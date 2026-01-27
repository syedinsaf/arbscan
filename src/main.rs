use std::env;
use std::fs::{File, write};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::mem::size_of;
use std::path::Path;

use serde::Serialize;

const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const EI_CLASS: usize = 4;
const EI_DATA: usize = 5;
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;

const HASH_HDR_SIZE: usize = 36;
const HASH_SCAN_MAX: usize = 0x1000;
const MAX_SEGMENT_SIZE: u64 = 20 * 1024 * 1024; // 20 MB safety cap

#[repr(C)]
#[derive(Clone, Copy)]
struct Elf64Phdr {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

#[derive(Serialize)]
struct ArbMetadata {
    device_model: String,
    update_label: String,

    image: String,
    major: u32,
    minor: u32,
    arb: u32,
    hash_offset: u64,
    hash_size: u64,
}

// helpers
fn read_le16(buf: &[u8], off: usize) -> u16 {
    u16::from_le_bytes(buf[off..off + 2].try_into().unwrap())
}
fn read_le32(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(buf[off..off + 4].try_into().unwrap())
}
fn read_le64(buf: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(buf[off..off + 8].try_into().unwrap())
}

fn sane_version(v: u32) -> bool {
    v < 1000
}

// ARB = 0 is VALID (OOS, OnePlus)
fn sane_arb(v: u32) -> bool {
    v < 128
}

fn ask_yes_no(prompt: &str) -> bool {
    print!("{}", prompt);
    let _ = io::stdout().flush();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

fn ask_string(prompt: &str) -> String {
    print!("{}", prompt);
    let _ = io::stdout().flush();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    input.trim().to_string()
}

fn json_filename(input: &str) -> String {
    let p = Path::new(input);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    format!("{}_arb.json", stem)
}

// HASH header detection
fn find_hash_header(seg: &[u8]) -> Option<usize> {
    for off in (0..HASH_SCAN_MAX.min(seg.len())).step_by(4) {
        if off + HASH_HDR_SIZE > seg.len() {
            break;
        }

        let version = read_le32(seg, off);
        let common_sz = read_le32(seg, off + 4) as usize;
        let qti_sz = read_le32(seg, off + 8) as usize;
        let oem_sz = read_le32(seg, off + 12) as usize;
        let hash_tbl_sz = read_le32(seg, off + 16) as usize;

        if !(1..=10).contains(&version) {
            continue;
        }
        if common_sz > 0x1000 || qti_sz > 0x1000 || oem_sz > 0x4000 {
            continue;
        }
        if hash_tbl_sz == 0 || (hash_tbl_sz & 0x1F) != 0 {
            continue;
        }
        if off + HASH_HDR_SIZE + common_sz + qti_sz + oem_sz > seg.len() {
            continue;
        }

        return Some(off);
    }
    None
}

// main
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .ok_or("usage: arbscan <xbl_config.img>")?;

    let mut file = File::open(&path)?;

    let mut ehdr = [0u8; 64];
    file.read_exact(&mut ehdr)?;

    if &ehdr[0..4] != ELF_MAGIC || ehdr[EI_CLASS] != ELFCLASS64 || ehdr[EI_DATA] != ELFDATA2LSB {
        return Err("Not a valid little-endian ELF64 file".into());
    }

    let e_phoff = read_le64(&ehdr, 0x20);
    let e_phentsz = read_le16(&ehdr, 0x36) as usize;
    let e_phnum = read_le16(&ehdr, 0x38) as usize;

    if e_phentsz < size_of::<Elf64Phdr>() || e_phnum == 0 {
        return Err("Unexpected program header layout".into());
    }

    let file_size = file.metadata()?.len();

    // Collect non-exec segment candidates
    let mut candidates = Vec::<(u64, u64)>::new();

    for i in 0..e_phnum {
        file.seek(SeekFrom::Start(e_phoff + (i as u64) * e_phentsz as u64))?;

        let mut buf = [0u8; size_of::<Elf64Phdr>()];
        file.read_exact(&mut buf)?;

        let ph = Elf64Phdr {
            p_type: read_le32(&buf, 0),
            p_flags: read_le32(&buf, 4),
            p_offset: read_le64(&buf, 8),
            p_vaddr: read_le64(&buf, 16),
            p_paddr: read_le64(&buf, 24),
            p_filesz: read_le64(&buf, 32),
            p_memsz: read_le64(&buf, 40),
            p_align: read_le64(&buf, 48),
        };

        if ph.p_filesz == 0 {
            continue;
        }
        if ph.p_offset + ph.p_filesz > file_size {
            continue;
        }
        if (ph.p_flags & 0x1) == 0
            && ph.p_filesz >= HASH_HDR_SIZE as u64
            && ph.p_filesz <= MAX_SEGMENT_SIZE
        {
            candidates.push((ph.p_offset, ph.p_filesz));
        }
    }

    // Select the correct HASH segment
    let mut seg = None;
    let mut header_off = None;
    let mut hash_off = 0u64;
    let mut hash_size = 0u64;

    for (off, size) in candidates {
        let mut buf = vec![0u8; size as usize];
        file.seek(SeekFrom::Start(off))?;
        file.read_exact(&mut buf)?;

        let Some(hdr) = find_hash_header(&buf) else {
            continue;
        };

        let oem_md_off = hdr
            + HASH_HDR_SIZE
            + read_le32(&buf, hdr + 4) as usize
            + read_le32(&buf, hdr + 8) as usize;

        if oem_md_off + 12 > buf.len() {
            continue;
        }

        let major = read_le32(&buf, oem_md_off);
        let minor = read_le32(&buf, oem_md_off + 4);
        let arb = read_le32(&buf, oem_md_off + 8);

        if sane_version(major) && sane_version(minor) && sane_arb(arb) {
            seg = Some(buf);
            header_off = Some(hdr);
            hash_off = off;
            hash_size = size;
            break;
        }
    }

    let seg = seg.ok_or("Valid OEM ARB metadata not found")?;
    let header_off = header_off.unwrap();

    let oem_md_off = header_off
        + HASH_HDR_SIZE
        + read_le32(&seg, header_off + 4) as usize
        + read_le32(&seg, header_off + 8) as usize;

    let major = read_le32(&seg, oem_md_off);
    let minor = read_le32(&seg, oem_md_off + 4);
    let arb = read_le32(&seg, oem_md_off + 8);

    println!("[arbscan] Analyzing: {}\n", path);
    println!("OEM Metadata");
    println!("────────────");
    println!("  Major Version : {}", major);
    println!("  Minor Version : {}", minor);
    println!("  ARB Index     : {}", arb);

    if ask_yes_no("\nWrite JSON output? [y/N]: ") {
        let device_model = ask_string("Device model      : ");
        let update_label = ask_string("Update / build    : ");

        let meta = ArbMetadata {
            device_model,
            update_label,
            image: path.clone(),
            major,
            minor,
            arb,
            hash_offset: hash_off,
            hash_size,
        };

        let out = json_filename(&path);
        write(&out, serde_json::to_string_pretty(&meta)?)?;
        println!("\n✔ JSON written: {}", out);
    }

    Ok(())
}

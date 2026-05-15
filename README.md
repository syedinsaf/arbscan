
<!-- markdownlint-configure-file {
  "MD033": false,
  "MD041": false
} -->
<div align="center">

# arbscan

A small, focused CLI tool to extract **OEM Anti-Rollback (ARB) metadata** from
Qualcomm bootloader images such as `xbl_config.img`.

[![GitHub release](https://img.shields.io/github/v/release/syedinsaf/arbscan?style=for-the-badge&logo=github&logoColor=white&color=rust)](https://github.com/syedinsaf/arbscan/releases)
[![Downloads](https://img.shields.io/github/downloads/syedinsaf/arbscan/total?style=for-the-badge&logo=github&logoColor=white&color=rust)](https://github.com/syedinsaf/arbscan/releases)
[![License](https://img.shields.io/github/license/syedinsaf/arbscan?style=for-the-badge&logo=github&logoColor=white&color=rust)](LICENSE)

</div>

---

> [!IMPORTANT]
> **🚀 ARCHIVE NOTICE: `arbscan` has been merged into [otaripper](https://github.com/syedinsaf/otaripper)!**
> 
> This standalone repository is no longer actively maintained. All ARB scanning capabilities have been natively integrated and massively upgraded in `otaripper` v3.0+.
> 
> By switching to `otaripper`, you gain the power of **Remote ARB Inspection (Zero-Download)**. You can now instantly check the ARB index of a firmware update by simply passing a direct HTTP URL to the OTA zip—`otaripper` will intelligently stream and extract just the tiny `xbl_config.img` over the internet **without you needing to download the massive 3GB+ firmware file!**
> 
> **Try it now:**
> ```bash
> otaripper arb https://example.com/firmware.zip -n
> ```
> 👉 **[Get the new otaripper here!](https://github.com/syedinsaf/otaripper/releases)**


## What arbscan does

`arbscan` parses a Qualcomm bootloader ELF image and extracts:

- OEM metadata **major / minor version**
- **Anti-Rollback (ARB) index**
- HASH segment offset and size
- Optional, user-provided context:
  - Device model
  - Update / build label
- Optional **JSON output** for archival or scripting

Parsing is implemented manually (no heavy ELF crates) to keep the binary small,
auditable, and predictable.

⚠️ Note:
arbscan is designed for **firmware analysis, research, and archival**.
This tool is **read-only**: it does not modify images and does not bypass secure
boot or rollback protection.

---

## What arbscan does *not* do

`arbscan` does **not**:

- Detect Android version
- Detect OTA / build number automatically
- Modify, patch, or re-sign firmware
- Bypass secure boot or rollback protection

⚠️ **Important**
The ARB value is **not an update counter**.
It is a **security rollback floor** enforced by the bootloader.

---

## Why ARB matters

The Anti-Rollback index answers a single question:

> *What is the oldest firmware generation this device will ever accept again?*

OEMs increment ARB when older firmware is permanently revoked.
Once increased, images with a lower rollback index will no longer boot.

This makes ARB useful for:
- Firmware research
- OTA comparison
- Understanding downgrade restrictions
- Long-term firmware archiving

---

## Interpreting ARB changes (example)

Consider the following real-world example:

* **COS 16.0.2.403**

  ```
  OEM Metadata Major Version : 3
  OEM Metadata Minor Version : 0
  ARB (Anti-Rollback)       : 0
  ```

* **COS 16.0.3.501**

  ```
  OEM Metadata Major Version : 3
  OEM Metadata Minor Version : 0
  ARB (Anti-Rollback)       : 1
  ```

This indicates that **COS 16.0.3.501 permanently raised the rollback index**.

### What this means

* The device will **no longer accept bootloader images with ARB < 1**
* Any attempt to boot or flash components from **16.0.2.403 (ARB 0)** after installing **16.0.3.501 (ARB 1)** will be **rejected by the bootloader**
* Downgrading firmware **below the raised ARB level is blocked by hardware-backed checks**

### Practical impact

* Flashing or downgrading to firmware with a **lower ARB** will:

  * Fail to boot, or
  * Be rejected during flashing, or
  * Leave the device in an **unbootable state** if mixed images are flashed

This is commonly referred to as a *brick*, but technically it is a **rollback enforcement failure**, not physical damage.

### Important clarification

* ARB is **not tied to Android version**
* ARB increases are **one-way**
* Once raised, ARB **cannot be lowered**, even with unlocked bootloaders

⚠️ **Rule of thumb:**
If a newer build increases ARB, **never downgrade bootloader-related images below that level**.

---

## Usage

```bash
arbscan [options] <xbl_config.img>
```

**Options:**
- `-h, --help`  : Print the help menu
- `--no-json`   : Disable interactive prompt for JSON output

Example:

```bash
arbscan xbl_config_pjz110_500update.img
```

Output:

```text
[arbscan] Analyzing: xbl_config_pjz110_500update.img

OEM Metadata
────────────
  Major Version : 3
  Minor Version : 0
  ARB Index     : 1
```

---

## Optional JSON output

After printing the metadata, `arbscan` can optionally write a JSON file.
If you wish to skip this prompt entirely in scripts, run the tool with the `--no-json` flag.

You will be prompted for:

* **Device model** (free-form, for your reference)
* **Update / build label** (free-form, for your reference)

Example:

```json
{
  "device_model": "PJZ110",
  "update_label": "OOS 16.0.500, Jan 2026 OTA",
  "image": "xbl_config_pjz110_500update.img",
  "major": 3,
  "minor": 0,
  "arb": 1,
  "hash_offset": 8388608,
  "hash_size": 65536
}
```

The file is written as:

```
<xbl_config>_arb.json
```

User-provided fields are **annotations only** and are not derived from firmware.

---

## Build

Requirements:

* Rust 1.95+ (edition 2024)

Build:

```bash
cargo build --release
```

Run:

```bash
./target/release/arbscan xbl_config.img
```

---

## Supported images

Primarily tested with:

* `xbl_config.img` (Qualcomm XBL)

Other Qualcomm bootloader images may work if they follow a similar HASH layout,
but the parser is intentionally conservative.

---

## Disclaimer

This project is for **educational and research purposes only**.

Do not use it to violate device security, terms of service, or local laws.
The author assumes no responsibility for misuse.

---

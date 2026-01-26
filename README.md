
<!-- markdownlint-configure-file {
  "MD033": false,
  "MD041": false
} -->
<div align="center">

# arbscan

</div>

A small, focused CLI tool to extract **OEM Anti-Rollback (ARB) metadata** from
Qualcomm bootloader images such as `xbl_config.img`.

Designed for **firmware analysis, research, and archival**.
This tool is **read-only**: it does not modify images and does not bypass secure
boot or rollback protection.

---

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

## Usage

```bash
arbscan <xbl_config.img>
```

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

* Rust 1.70+ (edition 2021)

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

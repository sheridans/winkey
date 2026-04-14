# winkey

[![CI](https://github.com/sheridans/winkey/actions/workflows/ci.yml/badge.svg)](https://github.com/sheridans/winkey/actions/workflows/ci.yml)

Extract the Windows product key from UEFI/BIOS firmware.

OEMs embed Windows licence keys in the ACPI MSDM table for activation. This tool reads that table and prints the key. Useful when setting up a Windows VM on Linux where the host already has a valid OEM licence baked into firmware.

## Usage

```
sudo winkey
```

Prints the product key to stdout. That's it.

```
sudo winkey -v
```

Prints the key to stdout and MSDM table metadata (OEM ID, checksums, SLS version, etc.) to stderr.

```
winkey --file /path/to/msdm.bin
```

Reads a raw MSDM binary dump instead of the system firmware. Works on any platform.

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Bad arguments |
| 2 | No MSDM table found |
| 3 | Permission denied (need root) |
| 4 | Parse error (corrupt table) |

### Piping

The key goes to stdout, everything else to stderr, so it works in scripts:

```sh
KEY=$(sudo winkey)
winkey --file dump.bin | xclip -selection clipboard
```

## Platform support

| Platform | Method | Dependencies |
|----------|--------|--------------|
| Linux | `/sys/firmware/acpi/tables/MSDM` | None |
| FreeBSD | `ioctl` on `/dev/acpi` | None (`libc`) |
| OpenBSD | `/var/db/acpi/MSDM` | None |
| Windows | Registry | `winreg` crate |
| macOS | Not supported | Use `--file` |

macOS machines don't have MSDM tables (Apple doesn't ship Windows OEM keys in firmware). Use `--file` if you have a raw dump.

## Building

```
cargo build --release
```

The binary has **zero dependencies on Linux/BSDs**. On Windows, the only dependency is `winreg` for registry access.

### Cross-compilation

```sh
# Static Linux binary (portable, ~500KB)
cargo build --release --target x86_64-unknown-linux-musl

# Windows
cargo build --release --target x86_64-pc-windows-msvc
```

## How it works

The ACPI MSDM (Microsoft Data Management) table is an 85-byte structure that OEMs write into UEFI firmware for Windows OEM Activation 3.0. The product key sits at byte offset 56 as a 29-character ASCII string (`XXXXX-XXXXX-XXXXX-XXXXX-XXXXX`).

On Linux, the kernel exposes ACPI tables as files under `/sys/firmware/acpi/tables/`. Reading the MSDM file requires root since the table permissions are `0400`.

## Why not just use `sudo cat /sys/firmware/acpi/tables/MSDM`?

You can. But the file is raw binary, so you'll get garbage mixed with the key. This tool parses the table properly, validates the key format, checks the ACPI checksum, and gives you just the key.

## Licence

MIT

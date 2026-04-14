mod msdm;
mod platform;

use std::process::ExitCode;

use msdm::MsdmError;
#[cfg(target_os = "windows")]
use msdm::ProductKey;

const EXIT_SUCCESS: u8 = 0;
const EXIT_USAGE: u8 = 1;
const EXIT_NOT_FOUND: u8 = 2;
const EXIT_PERMISSION: u8 = 3;
const EXIT_PARSE_ERROR: u8 = 4;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE: &str = "\
winkey - Extract the Windows product key from UEFI/BIOS firmware

Usage: winkey [OPTIONS]

Options:
  -v, --verbose        Show MSDM table metadata alongside the product key
  -f, --file <PATH>    Read a raw MSDM binary dump from a file instead of firmware
  -h, --help           Show this help message
  -V, --version        Show the version number";

struct Args {
    verbose: bool,
    file_path: Option<String>,
}

fn parse_args() -> Result<Args, ExitCode> {
    let mut verbose = false;
    let mut file_path = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-v" | "--verbose" => verbose = true,
            "-f" | "--file" => {
                let path = args.next().ok_or_else(|| {
                    eprintln!("Error: --file requires a path argument.");
                    eprintln!();
                    eprintln!("{USAGE}");
                    ExitCode::from(EXIT_USAGE)
                })?;
                file_path = Some(path);
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return Err(ExitCode::from(EXIT_SUCCESS));
            }
            "-V" | "--version" => {
                println!("winkey {VERSION}");
                return Err(ExitCode::from(EXIT_SUCCESS));
            }
            other => {
                eprintln!("Unknown option: {other}");
                eprintln!();
                eprintln!("{USAGE}");
                return Err(ExitCode::from(EXIT_USAGE));
            }
        }
    }

    Ok(Args { verbose, file_path })
}

fn exit_code_for_error(err: &MsdmError) -> ExitCode {
    match err {
        MsdmError::NotFound => ExitCode::from(EXIT_NOT_FOUND),
        MsdmError::PermissionDenied => ExitCode::from(EXIT_PERMISSION),
        _ => ExitCode::from(EXIT_PARSE_ERROR),
    }
}

fn run(args: &Args) -> Result<(), MsdmError> {
    let result = if let Some(ref path) = args.file_path {
        let bytes = std::fs::read(path)?;
        platform::PlatformResult::RawTable(bytes)
    } else {
        platform::read_msdm()?
    };

    match result {
        platform::PlatformResult::RawTable(bytes) => {
            let table = msdm::parse_table(&bytes)?;

            println!("{}", table.product_key);

            if args.verbose {
                if !table.checksum_valid {
                    eprintln!("Warning: ACPI checksum mismatch. Possible firmware bug.");
                    eprintln!();
                }
                eprintln!("MSDM table details:");
                eprintln!("  Table length:      {} bytes", table.length);
                eprintln!("  Revision:          {}", table.revision);
                eprintln!(
                    "  Checksum:          0x{:02X} ({})",
                    table.checksum,
                    if table.checksum_valid {
                        "valid"
                    } else {
                        "invalid"
                    }
                );
                eprintln!("  OEM ID:            {}", table.oem_id);
                eprintln!("  OEM Table ID:      {}", table.oem_table_id);
                eprintln!("  OEM Revision:      {}", table.oem_revision);
                eprintln!("  Creator ID:        {}", table.creator_id);
                eprintln!("  Creator Revision:  {}", table.creator_revision);
                eprintln!("  SLS Version:       {}", table.sls_version);
                eprintln!("  SLS Data Type:     {}", table.sls_data_type);
                eprintln!("  SLS Data Length:    {}", table.sls_data_length);
            }
        }
        #[cfg(target_os = "windows")]
        platform::PlatformResult::KeyOnly(key_str) => {
            let key = ProductKey::new(&key_str)?;
            println!("{key}");

            if args.verbose {
                eprintln!("Source: Windows registry (full MSDM table metadata not available).");
            }
        }
    }

    Ok(())
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(args) => args,
        Err(code) => return code,
    };

    match run(&args) {
        Ok(()) => ExitCode::from(EXIT_SUCCESS),
        Err(err) => {
            eprintln!("Error: {err}");
            exit_code_for_error(&err)
        }
    }
}

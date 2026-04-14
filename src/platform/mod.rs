use crate::msdm::MsdmError;

pub enum PlatformResult {
    /// Raw MSDM binary table (Linux, FreeBSD, OpenBSD).
    RawTable(Vec<u8>),
    /// Key string only, from Windows registry.
    #[cfg(target_os = "windows")]
    KeyOnly(String),
}

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "freebsd")]
mod freebsd;

#[cfg(target_os = "openbsd")]
mod openbsd;

#[cfg(target_os = "windows")]
mod windows;

/// Reads the MSDM table from system firmware.
pub fn read_msdm() -> Result<PlatformResult, MsdmError> {
    #[cfg(target_os = "linux")]
    {
        linux::read_msdm_bytes().map(PlatformResult::RawTable)
    }

    #[cfg(target_os = "freebsd")]
    {
        freebsd::read_msdm_bytes().map(PlatformResult::RawTable)
    }

    #[cfg(target_os = "openbsd")]
    {
        openbsd::read_msdm_bytes().map(PlatformResult::RawTable)
    }

    #[cfg(target_os = "windows")]
    {
        windows::read_msdm()
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "windows"
    )))]
    {
        compile_error!(
            "winkey does not support this platform natively. \
             Use --file to read a raw MSDM dump instead."
        );
    }
}

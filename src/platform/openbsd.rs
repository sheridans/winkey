use crate::msdm::MsdmError;

const ACPIDUMP_MSDM_PATH: &str = "/var/db/acpi/MSDM";

/// Fallback path.
const ACPIDUMP_MSDM_ALT_PATH: &str = "/var/db/acpidump/MSDM";

/// Reads the raw MSDM table from `acpidump(8)` output.
/// OpenBSD lacks sysfs or ioctl access to ACPI tables. Tries both known paths.
pub fn read_msdm_bytes() -> Result<Vec<u8>, MsdmError> {
    match std::fs::read(ACPIDUMP_MSDM_PATH) {
        Ok(bytes) => Ok(bytes),
        Err(primary_err) => match std::fs::read(ACPIDUMP_MSDM_ALT_PATH) {
            Ok(bytes) => Ok(bytes),
            Err(_) => Err(MsdmError::from(primary_err)),
        },
    }
}

use crate::msdm::MsdmError;

const MSDM_SYSFS_PATH: &str = "/sys/firmware/acpi/tables/MSDM";

/// Read the raw MSDM ACPI table from the Linux sysfs interface.
pub fn read_msdm_bytes() -> Result<Vec<u8>, MsdmError> {
    Ok(std::fs::read(MSDM_SYSFS_PATH)?)
}

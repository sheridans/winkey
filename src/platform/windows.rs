use crate::msdm::MsdmError;
use crate::platform::PlatformResult;

const REGISTRY_KEY_PATH: &str =
    r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\SoftwareProtectionPlatform";
const REGISTRY_VALUE_NAME: &str = "BackupProductKeyDefault";

/// Reads the OEM product key from the registry.
/// Returns the key string only, not full MSDM table metadata.
pub fn read_msdm() -> Result<PlatformResult, MsdmError> {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    let subkey = hklm.open_subkey(REGISTRY_KEY_PATH).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            MsdmError::NotFound
        } else if e.kind() == std::io::ErrorKind::PermissionDenied {
            MsdmError::PermissionDenied
        } else {
            MsdmError::Registry(e.to_string())
        }
    })?;

    let key: String = subkey.get_value(REGISTRY_VALUE_NAME).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            MsdmError::NotFound
        } else {
            MsdmError::Registry(e.to_string())
        }
    })?;

    if key.is_empty() {
        return Err(MsdmError::NotFound);
    }

    Ok(PlatformResult::KeyOnly(key))
}

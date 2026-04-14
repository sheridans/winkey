use std::fmt;

// ACPI MSDM table layout constants
const MSDM_SIGNATURE: &[u8; 4] = b"MSDM";

// ACPI standard header field offsets
const OFFSET_SIGNATURE: usize = 0;
const OFFSET_LENGTH: usize = 4;
const OFFSET_REVISION: usize = 8;
const OFFSET_CHECKSUM: usize = 9;
const OFFSET_OEM_ID: usize = 10;
const OFFSET_OEM_TABLE_ID: usize = 16;
const OFFSET_OEM_REVISION: usize = 24;
const OFFSET_CREATOR_ID: usize = 28;
const OFFSET_CREATOR_REVISION: usize = 32;

// Software Licensing Structure (SLS) field offsets
const OFFSET_SLS_VERSION: usize = 36;
const OFFSET_SLS_DATA_TYPE: usize = 44;
const OFFSET_SLS_DATA_LENGTH: usize = 52;
const OFFSET_PRODUCT_KEY: usize = 56;

// Field sizes
const OEM_ID_LENGTH: usize = 6;
const OEM_TABLE_ID_LENGTH: usize = 8;
const CREATOR_ID_LENGTH: usize = 4;

// Product key format
const PRODUCT_KEY_LENGTH: usize = 29;
const PRODUCT_KEY_GROUP_COUNT: usize = 5;
const PRODUCT_KEY_GROUP_LENGTH: usize = 5;
const PRODUCT_KEY_SEPARATOR: char = '-';

/// Minimum valid MSDM table size: ACPI header (36) + SLS fields (20) + key (29)
const MSDM_TABLE_MIN_LENGTH: usize = OFFSET_PRODUCT_KEY + PRODUCT_KEY_LENGTH;

/// Parsed MSDM (Microsoft Data Management) ACPI table.
#[derive(Debug)]
pub struct MsdmTable {
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: String,
    pub oem_table_id: String,
    pub oem_revision: u32,
    pub creator_id: String,
    pub creator_revision: u32,
    pub sls_version: u32,
    pub sls_data_type: u32,
    pub sls_data_length: u32,
    pub product_key: ProductKey,
    pub checksum_valid: bool,
}

/// A validated Windows product key in the format `XXXXX-XXXXX-XXXXX-XXXXX-XXXXX`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductKey(String);

impl ProductKey {
    /// Validate and wrap a product key string.
    pub fn new(raw: &str) -> Result<Self, MsdmError> {
        let key = raw.trim().trim_end_matches('\0');
        validate_key_format(key)?;
        Ok(Self(key.to_uppercase()))
    }
}

impl AsRef<str> for ProductKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProductKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug)]
pub enum MsdmError {
    NotFound,
    PermissionDenied,
    InvalidSignature {
        got: [u8; 4],
    },
    TableTooShort {
        expected: usize,
        got: usize,
    },
    InvalidUtf8 {
        field: &'static str,
    },
    InvalidKeyFormat {
        key: String,
        reason: &'static str,
    },
    Io(std::io::Error),
    #[cfg(target_os = "windows")]
    Registry(String),
}

impl fmt::Display for MsdmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(
                f,
                "MSDM table not found: no Windows product key in firmware."
            ),
            Self::PermissionDenied => write!(
                f,
                "Permission denied: reading the MSDM table requires root \
                 privileges. Try running with sudo."
            ),
            Self::InvalidSignature { got } => {
                let sig = String::from_utf8_lossy(got);
                write!(
                    f,
                    "Invalid MSDM table: expected signature \"MSDM\", got \"{sig}\"."
                )
            }
            Self::TableTooShort { expected, got } => write!(
                f,
                "Invalid MSDM table: expected at least {expected} bytes, got {got}."
            ),
            Self::InvalidUtf8 { field } => {
                write!(f, "Invalid MSDM table: {field} contains invalid UTF-8.")
            }
            Self::InvalidKeyFormat { key, reason } => {
                write!(f, "Invalid product key \"{key}\": {reason}.")
            }
            Self::Io(err) => write!(f, "I/O error: {err}"),
            #[cfg(target_os = "windows")]
            Self::Registry(msg) => write!(f, "Registry error: {msg}"),
        }
    }
}

impl std::error::Error for MsdmError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for MsdmError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => Self::NotFound,
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            _ => Self::Io(err),
        }
    }
}

/// Reads a little-endian u32 at `offset`. Caller must bounds-check first.
fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

/// Extract a trimmed ASCII string from a byte slice.
fn read_ascii_lossy(bytes: &[u8], offset: usize, length: usize) -> String {
    String::from_utf8_lossy(&bytes[offset..offset + length])
        .trim()
        .trim_end_matches('\0')
        .to_string()
}

/// Parse a raw MSDM ACPI table from bytes.
pub fn parse_table(bytes: &[u8]) -> Result<MsdmTable, MsdmError> {
    if bytes.len() < MSDM_TABLE_MIN_LENGTH {
        return Err(MsdmError::TableTooShort {
            expected: MSDM_TABLE_MIN_LENGTH,
            got: bytes.len(),
        });
    }

    let sig = [
        bytes[OFFSET_SIGNATURE],
        bytes[OFFSET_SIGNATURE + 1],
        bytes[OFFSET_SIGNATURE + 2],
        bytes[OFFSET_SIGNATURE + 3],
    ];
    if sig != *MSDM_SIGNATURE {
        return Err(MsdmError::InvalidSignature { got: sig });
    }

    // Verify ACPI checksum: all bytes should sum to zero (mod 256)
    let checksum_valid = bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b)) == 0;

    // Extract product key as strict UTF-8
    let key_bytes = &bytes[OFFSET_PRODUCT_KEY..OFFSET_PRODUCT_KEY + PRODUCT_KEY_LENGTH];
    let key_str = std::str::from_utf8(key_bytes).map_err(|_| MsdmError::InvalidUtf8 {
        field: "product key",
    })?;

    let product_key = ProductKey::new(key_str)?;

    Ok(MsdmTable {
        length: read_u32_le(bytes, OFFSET_LENGTH),
        revision: bytes[OFFSET_REVISION],
        checksum: bytes[OFFSET_CHECKSUM],
        oem_id: read_ascii_lossy(bytes, OFFSET_OEM_ID, OEM_ID_LENGTH),
        oem_table_id: read_ascii_lossy(bytes, OFFSET_OEM_TABLE_ID, OEM_TABLE_ID_LENGTH),
        oem_revision: read_u32_le(bytes, OFFSET_OEM_REVISION),
        creator_id: read_ascii_lossy(bytes, OFFSET_CREATOR_ID, CREATOR_ID_LENGTH),
        creator_revision: read_u32_le(bytes, OFFSET_CREATOR_REVISION),
        sls_version: read_u32_le(bytes, OFFSET_SLS_VERSION),
        sls_data_type: read_u32_le(bytes, OFFSET_SLS_DATA_TYPE),
        sls_data_length: read_u32_le(bytes, OFFSET_SLS_DATA_LENGTH),
        product_key,
        checksum_valid,
    })
}

/// Validate that a product key matches the format `XXXXX-XXXXX-XXXXX-XXXXX-XXXXX`.
fn validate_key_format(key: &str) -> Result<(), MsdmError> {
    let make_err = |reason: &'static str| MsdmError::InvalidKeyFormat {
        key: key.to_string(),
        reason,
    };

    let expected_length =
        PRODUCT_KEY_GROUP_COUNT * PRODUCT_KEY_GROUP_LENGTH + (PRODUCT_KEY_GROUP_COUNT - 1);

    if key.len() != expected_length {
        return Err(make_err(
            "key must be 29 characters (XXXXX-XXXXX-XXXXX-XXXXX-XXXXX)",
        ));
    }

    let mut group_count = 0usize;
    for group in key.split(PRODUCT_KEY_SEPARATOR) {
        group_count += 1;
        if group_count > PRODUCT_KEY_GROUP_COUNT {
            return Err(make_err(
                "key must have exactly 5 groups separated by hyphens",
            ));
        }
        if group.len() != PRODUCT_KEY_GROUP_LENGTH {
            return Err(make_err("each group must be exactly 5 characters"));
        }
        if !group.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(make_err(
                "key must contain only alphanumeric characters and hyphens",
            ));
        }
    }
    if group_count != PRODUCT_KEY_GROUP_COUNT {
        return Err(make_err(
            "key must have exactly 5 groups separated by hyphens",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic valid MSDM table for testing.
    fn build_test_table(key: &str) -> Vec<u8> {
        let mut table = vec![0u8; MSDM_TABLE_MIN_LENGTH];

        // Signature
        table[OFFSET_SIGNATURE..OFFSET_SIGNATURE + 4].copy_from_slice(b"MSDM");

        // Length
        let length = MSDM_TABLE_MIN_LENGTH as u32;
        table[OFFSET_LENGTH..OFFSET_LENGTH + 4].copy_from_slice(&length.to_le_bytes());

        // Revision
        table[OFFSET_REVISION] = 1;

        // OEM ID
        table[OFFSET_OEM_ID..OFFSET_OEM_ID + 6].copy_from_slice(b"ALASKA");

        // OEM Table ID
        table[OFFSET_OEM_TABLE_ID..OFFSET_OEM_TABLE_ID + 4].copy_from_slice(b"A M ");

        // Creator ID
        table[OFFSET_CREATOR_ID..OFFSET_CREATOR_ID + 4].copy_from_slice(b"AMI ");

        // SLS version
        table[OFFSET_SLS_VERSION..OFFSET_SLS_VERSION + 4].copy_from_slice(&1u32.to_le_bytes());

        // SLS data type
        table[OFFSET_SLS_DATA_TYPE..OFFSET_SLS_DATA_TYPE + 4].copy_from_slice(&1u32.to_le_bytes());

        // SLS data length
        let key_len = key.len() as u32;
        table[OFFSET_SLS_DATA_LENGTH..OFFSET_SLS_DATA_LENGTH + 4]
            .copy_from_slice(&key_len.to_le_bytes());

        // Product key
        table[OFFSET_PRODUCT_KEY..OFFSET_PRODUCT_KEY + key.len()].copy_from_slice(key.as_bytes());

        // Fix checksum so all bytes sum to 0 mod 256
        let sum: u8 = table.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        table[OFFSET_CHECKSUM] = 0u8.wrapping_sub(sum);

        table
    }

    mod parse_table {
        use super::*;

        #[test]
        fn extracts_product_key() {
            let key = "ABCDE-12345-FGHIJ-67890-KLMNO";
            let table = parse_table(&build_test_table(key)).unwrap();
            assert_eq!(table.product_key.as_ref(), key);
        }

        #[test]
        fn extracts_table_length() {
            let table = parse_table(&build_test_table("ABCDE-12345-FGHIJ-67890-KLMNO")).unwrap();
            assert_eq!(table.length, MSDM_TABLE_MIN_LENGTH as u32);
        }

        #[test]
        fn extracts_oem_id() {
            let table = parse_table(&build_test_table("ABCDE-12345-FGHIJ-67890-KLMNO")).unwrap();
            assert_eq!(table.oem_id, "ALASKA");
        }

        #[test]
        fn validates_checksum() {
            let table = parse_table(&build_test_table("ABCDE-12345-FGHIJ-67890-KLMNO")).unwrap();
            assert!(table.checksum_valid);
        }

        #[test]
        fn uppercases_lowercase_key() {
            let table = parse_table(&build_test_table("abcde-12345-fghij-67890-klmno")).unwrap();
            assert_eq!(table.product_key.as_ref(), "ABCDE-12345-FGHIJ-67890-KLMNO");
        }

        #[test]
        fn returns_error_when_table_too_short() {
            let err = parse_table(&[0u8; 50]).unwrap_err();
            assert!(
                matches!(
                    err,
                    MsdmError::TableTooShort {
                        expected: 85,
                        got: 50
                    }
                ),
                "expected TableTooShort, got: {err}"
            );
        }

        #[test]
        fn returns_error_when_signature_wrong() {
            let mut bytes = build_test_table("ABCDE-12345-FGHIJ-67890-KLMNO");
            bytes[0..4].copy_from_slice(b"HPET");

            let err = parse_table(&bytes).unwrap_err();
            assert!(
                matches!(err, MsdmError::InvalidSignature { got } if &got == b"HPET"),
                "expected InvalidSignature, got: {err}"
            );
        }

        #[test]
        fn detects_bad_checksum() {
            let mut bytes = build_test_table("ABCDE-12345-FGHIJ-67890-KLMNO");
            bytes[OFFSET_CHECKSUM] = bytes[OFFSET_CHECKSUM].wrapping_add(1);

            let table = parse_table(&bytes).unwrap();
            assert!(!table.checksum_valid);
        }

        #[test]
        fn returns_error_when_key_not_utf8() {
            let mut bytes = build_test_table("ABCDE-12345-FGHIJ-67890-KLMNO");
            bytes[OFFSET_PRODUCT_KEY] = 0xFF;
            bytes[OFFSET_PRODUCT_KEY + 1] = 0xFE;

            let err = parse_table(&bytes).unwrap_err();
            assert!(matches!(
                err,
                MsdmError::InvalidUtf8 {
                    field: "product key"
                }
            ));
        }
    }

    mod product_key {
        use super::*;

        #[test]
        fn accepts_alphanumeric_key() {
            assert!(ProductKey::new("ABCDE-12345-FGHIJ-67890-KLMNO").is_ok());
        }

        #[test]
        fn accepts_all_letters() {
            assert!(ProductKey::new("XXXXX-XXXXX-XXXXX-XXXXX-XXXXX").is_ok());
        }

        #[test]
        fn accepts_mixed_alphanumeric() {
            assert!(ProductKey::new("A1B2C-D3E4F-G5H6I-J7K8L-M9N0O").is_ok());
        }

        #[test]
        fn rejects_wrong_length() {
            let err = ProductKey::new("ABCDE-12345").unwrap_err();
            assert!(matches!(err, MsdmError::InvalidKeyFormat { .. }));
        }

        #[test]
        fn rejects_wrong_separator() {
            let err = ProductKey::new("ABCDE_12345_FGHIJ_67890_KLMNO").unwrap_err();
            assert!(matches!(err, MsdmError::InvalidKeyFormat { .. }));
        }

        #[test]
        fn rejects_special_characters() {
            let err = ProductKey::new("ABCD!-12345-FGHIJ-67890-KLMNO").unwrap_err();
            assert!(matches!(err, MsdmError::InvalidKeyFormat { .. }));
        }

        #[test]
        fn trims_trailing_nulls() {
            let key = ProductKey::new("ABCDE-12345-FGHIJ-67890-KLMNO\0\0").unwrap();
            assert_eq!(key.as_ref(), "ABCDE-12345-FGHIJ-67890-KLMNO");
        }
    }
}

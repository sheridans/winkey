use crate::msdm::MsdmError;
use std::fs::File;
use std::os::unix::io::AsRawFd;

const ACPI_DEVICE_PATH: &str = "/dev/acpi";
const MSDM_TABLE_NAME: &[u8; 4] = b"MSDM";

/// Maximum expected size of an MSDM table.
const MAX_TABLE_SIZE: usize = 4096;

// _IOWR('P', 3, struct acpi_table_header)
const ACPIIO_FINDTABLE: libc::c_ulong = 0xC0105003;

/// Mirrors `struct acpi_find_table` from `/usr/include/dev/acpica/acpiio.h`.
#[repr(C)]
struct AcpiFindTable {
    /// 4-character signature of the table to find (e.g. "MSDM").
    signature: [u8; 4],
    /// On input: which instance (0 = first). On output: unchanged.
    instance: u32,
    /// On output: pointer to table data.
    data: *mut u8,
    /// On input: buffer size. On output: actual table size.
    length: u32,
}

/// Read the raw MSDM ACPI table via FreeBSD's `/dev/acpi` ioctl interface.
pub fn read_msdm_bytes() -> Result<Vec<u8>, MsdmError> {
    let file = File::open(ACPI_DEVICE_PATH)?;
    let fd = file.as_raw_fd();

    let mut buffer = vec![0u8; MAX_TABLE_SIZE];

    let mut request = AcpiFindTable {
        signature: *MSDM_TABLE_NAME,
        instance: 0,
        data: buffer.as_mut_ptr(),
        length: buffer.len() as u32,
    };

    // SAFETY: request matches the kernel ABI, buffer is valid for MAX_TABLE_SIZE
    // bytes, fd is an open handle to /dev/acpi.
    let result = unsafe { libc::ioctl(fd, ACPIIO_FINDTABLE, &mut request) };

    if result < 0 {
        let err = std::io::Error::last_os_error();
        return Err(MsdmError::from(err));
    }

    if request.length == 0 {
        return Err(MsdmError::NotFound);
    }

    buffer.truncate(request.length as usize);
    Ok(buffer)
}

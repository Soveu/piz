use core::mem;
use core::ops::Range;

pub const CENTRAL_DIR_END_SIGNATURE: u32 = 0x06054b50;
pub const CENTRAL_DIR_END_SIGNATURE_ZIP64: u32 = 0x06064b50;
pub const CENTRAL_DIR_HEADER_SIGNATURE: u32 = 0x02014b50;

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct LocalFileHeader {
    pub signature: u32, // 0x04034b50
    pub version_min: u16,
    pub flags: u16,
    pub compression_method: u16,
    pub last_mod_time: u16,
    pub last_mod_date: u16,
    pub uncompressed_crc: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub filename_len: u16,
    pub extra_field_len: u16,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct CentralDirectoryFileHeader {
    pub signature: u32, // 0x02014b50
    pub version_made_by: u16,
    pub version_min: u16,
    pub flags: u16,
    pub compression_method: u16,
    pub last_mod_time: u16,
    pub last_mod_date: u16,
    pub decompressed_crc: u32,
    pub compressed_size: u32,
    pub decompressed_size: u32,
    pub filename_len: u16,
    pub extra_field_len: u16,
    pub file_comment_len: u16,
    pub disk_number: u16,
    pub file_attr_internal: u16,
    pub file_attr_external: u32,
    pub local_file_header_offset: u32,
}

/// The actual ZIP "header"
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct CentralDirectoryRecordEnd {
    pub signature: u32, // 0x06054b50
    pub disk_num: u16,
    pub central_dir_start_disk: u16,
    pub central_dir_records_on_this_disk: u16,
    pub central_dir_records_total: u16,
    pub central_dir_size: u32,
    pub central_dir_offset: u32,
    pub comment_length: u16,
}

#[repr(C, packed)]
pub struct ExtraHeader {
    pub header: u16,
    pub extra_field_size: u16,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Zip64Extra {
    pub decompressed_size: u64,
    pub compressed_size: u64,
    pub local_header_record_offset: u64,
    pub disk_no: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Zip64CentralDirectoryRecordEnd {
    pub signature: u32, // 0x06064b50
    pub self_size: u64,
    pub version_made_by: u16,
    pub version_min: u16,
    pub disk_num: u32,
    pub central_dir_start_disk: u32,
    pub central_dir_records_on_this_disk: u64,
    pub central_dir_records_total: u64,
    pub central_directory_size: u64,
    pub central_dir_offset: u64,
}

impl CentralDirectoryRecordEnd {
    const SELF_SIZE: usize = mem::size_of::<Self>();

    /// Finds ZIP "header" with a little twist.
    /// Because it is possible to embed a valid zip header inside zip header comment,
    /// this tries to find the topmost header.
    /// This returns reference to CentralDirectoryRecordEnd and length of the comment
    pub fn find(bytes: &[u8]) -> Option<(&Self, usize)> {
        let start_offset = bytes.len().saturating_sub(u16::MAX as usize);
        let bytes = &bytes[start_offset..];

        /* SAFETY: Self is so-called "Plain Ol' Data", so we can cast from bytes
         * to an actual struct pointer.
         * `array_windows` cares about having enough bytes. */
        bytes
            .array_windows::<{Self::SELF_SIZE}>()
            .rev()
            .map(|window| unsafe { &*(window as *const _ as *const Self) })
            .enumerate()
            .filter(|&(_i, maybe_header)| maybe_header.signature == CENTRAL_DIR_END_SIGNATURE)
            .filter(|&(i, maybe_header)| maybe_header.comment_length as usize == i)
            .next()
            .map(|t| (t.1, t.0))
    }

    pub fn central_dir_range(&self) -> Range<usize> {
        let offset = self.central_dir_offset as usize;
        let size = self.central_dir_size as usize;
        offset .. offset+size
    }
}


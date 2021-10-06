#![no_std]

#![feature(array_windows)]

use core::mem;
use core::ops::Range;

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
    pub uncompressed_crc: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
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
pub struct EndOfCentralDirectoryRecord {
    pub signature: u32, // 0x06054b50
    pub disk_num: u16,
    pub central_dir_start_disk: u16,
    pub central_dir_records_on_this_disk: u16,
    pub central_dir_records_total: u16,
    pub central_dir_size: u32,
    pub central_dir_offset: u32,
    pub comment_length: u16,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Zip64Extra {
    pub header: u16, // 0x0001
    pub extra_field_size: u16,
    pub uncompressed_filesize: u64,
    pub compressed_data_size: u64,
    pub local_header_record_offset: u64,
    pub disk_no: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Zip64EndOfCentralDirectoryRecord {
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

pub const CENTRAL_DIR_END_SIGNATURE: u32 = 0x06054b50;
pub const CENTRAL_DIR_END_SIGNATURE_ZIP64: u32 = 0x06064b50;
pub const CENTRAL_DIR_HEADER_SIGNATURE: u32 = 0x02014b50;

impl EndOfCentralDirectoryRecord {
    const SELF_SIZE: usize = mem::size_of::<Self>();

    /// Finds ZIP "header" with a little twist.
    /// Because it is possible to embed a valid zip header inside zip header comment,
    /// this tries to find the topmost header.
    pub fn find(bytes: &[u8]) -> Option<&Self> {
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
            .last()
            .map(|(_i, maybe_header)| maybe_header)
    }

    pub fn central_dir_range(&self) -> Range<usize> {
        let offset = self.central_dir_offset as usize;
        let size = self.central_dir_size as usize;
        offset .. offset+size
    }
}

fn slice_split_at<T>(s: &[T], index: usize) -> Option<(&[T], &[T])> {
    if index > s.len() {
        return None;
    }

    /* SAFETY: we checked the bounds */
    return unsafe {
        Some((s.get_unchecked(..index), s.get_unchecked(index..)))
    };
}

/// Iterator over CentralDirIter.
///
/// It is not strict, meaning that if for example signature is invalid or there
/// is a different number of directories than header says, it will just work.
///
/// On iteration, it returns a `(central dir, additional data slice)` tuple, so if you need
/// for example filename, you need to grab it yourself
/* TODO: make a stricter version of this iterator, that fails on these
 * - signature doesn't match
 * - directory has too much / too little records 
 * - some other weird stuff */
pub struct CentralDirIter<'a> {
    /// Place in memory, where directories are contiguously allocated
    pub data: &'a [u8],

    //pub _remaining_items: usize,
}

impl<'a> Iterator for CentralDirIter<'a> {
    type Item = (&'a CentralDirectoryFileHeader, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        const CENTRAL_DIR_HEADER_SIZE: usize = mem::size_of::<CentralDirectoryFileHeader>();

        let (central_dir, bytes) = slice_split_at(self.data, CENTRAL_DIR_HEADER_SIZE)?;
        let central_dir = central_dir.as_ptr() as *const CentralDirectoryFileHeader;
        /* SAFETY: Again something that `bytemuck` crate would handle nicer, but in
         * the same way - we have enough bytes and they're aligned enough to be casted */
        let central_dir = unsafe { &*central_dir };

        debug_assert_eq!({central_dir.signature}, CENTRAL_DIR_HEADER_SIGNATURE);

        let bonus_bytes_len =
            central_dir.filename_len as usize +
            central_dir.extra_field_len as usize +
            central_dir.file_comment_len as usize;
        let (bonus_bytes, bytes) = slice_split_at(bytes, bonus_bytes_len)?;

        self.data = bytes;

        return Some((central_dir, bonus_bytes));
    }
}


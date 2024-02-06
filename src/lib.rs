#![no_std]

#![feature(array_windows)]

use core::mem;

pub mod raw;
pub mod extra;

use crate::extra::Extra;

pub struct Zip<'data> {
    pub central_dir_iter: NonStrictIter<'data>,
    pub central_dir_records_total: u64,
}

impl<'data> Zip<'data> {
    pub fn new(data: &'data [u8]) -> Option<Self> {
        let (header, _comment_len) = raw::CentralDirectoryRecordEnd::find(data)?;

        // TODO: zip64
        let central_dir_records_total = header.central_dir_records_total as u64;
        let central_dir_iter = NonStrictIter {
            data,
            offset: header.central_dir_offset as usize,
        };

        Some(Self {
            central_dir_records_total,
            central_dir_iter,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CompressionMethod {
    Plain = 0,

    Deflate = 8,
    Deflate64 = 9,
    IbmTerseOld = 10,
    Bzip2 = 12,
    Lzma = 14,
    IbmCmpsc = 16,
    IbmTerseNew = 18,
    IbmLz77 = 19,

    Zstd = 93,
    Mp3 = 94,
    Xz = 95,
    Jpeg = 96,
    WavPack = 97,
    Ppmd1 = 98,
}

impl CompressionMethod {
    pub const fn from_u16(x: u16) -> Option<Self> {
        let ret = match x {
            0 => Self::Plain,
            8 => Self::Deflate,
            _ => return None,
        };
        return Some(ret);
    }
}

#[derive(Debug)]
pub struct ExtraFields;

#[derive(Debug)]
pub struct File<'data> {
    pub decompressed_crc: u32,
    pub decompressed_size: usize,
    pub compression_method: CompressionMethod,
    pub extra_fields: &'data [u8],
    pub filename: &'data [u8],
    pub comment: &'data [u8],
    pub bytes: &'data [u8],
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
pub struct NonStrictIter<'a> {
    pub data: &'a [u8],
    pub offset: usize,

    //pub _remaining_items: usize,
}

impl<'a> Iterator for NonStrictIter<'a> {
    type Item = File<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        const CENTRAL_DIR_HEADER_SIZE: usize = mem::size_of::<raw::CentralDirectoryFileHeader>();

        let bytes = self.data.get(self.offset..)?;
        let (central_dir, bytes) = slice_split_at(bytes, CENTRAL_DIR_HEADER_SIZE)?;
        let central_dir = central_dir.as_ptr() as *const raw::CentralDirectoryFileHeader;
        /* SAFETY: Again something that `bytemuck` crate would handle nicer, but in
         * the same way - we have enough bytes and they're aligned enough to be casted */
        let central_dir = unsafe { &*central_dir };

        //debug_assert_eq!({central_dir.signature}, CENTRAL_DIR_HEADER_SIGNATURE);

        let mut local_file_offset = central_dir.local_file_header_offset as usize;
        let mut compressed_size = central_dir.compressed_size as usize;
        let mut decompressed_size = central_dir.decompressed_size as usize;
        let filename_len = central_dir.filename_len as usize;
        let extra_fields_len = central_dir.extra_field_len as usize;
        let file_comment_len = central_dir.file_comment_len as usize;

        let (filename, bytes) = slice_split_at(bytes, filename_len)?;
        let (extra_fields, bytes) = slice_split_at(bytes, extra_fields_len)?; // TODO: parse extra fields
        let (comment, _bytes) = slice_split_at(bytes, file_comment_len)?;

        let mut extra_iter = extra::Iter { data: extra_fields };
        if let Some(zip64) = extra_iter
            .find(|(signature, _)| *signature == extra::Zip64::SIGNATURE)
            .and_then(|(_, data)| extra::Zip64::parse(data))
        {
            compressed_size = zip64.compressed_size as usize;
            decompressed_size = zip64.decompressed_size as usize;
            local_file_offset = zip64.local_header_record_offset as usize;
        }

        let total_bytes_len =
            CENTRAL_DIR_HEADER_SIZE +
            filename_len as usize +
            extra_fields_len as usize +
            file_comment_len as usize;

        self.offset += total_bytes_len;

        let local_file_header = self.data.get(local_file_offset..)
            .and_then(|slice| slice.get(.. mem::size_of::<raw::LocalFileHeader>()))?;
        let local_file_header = local_file_header.as_ptr() as *const raw::LocalFileHeader;
        /* SAFETY: Again something that `bytemuck` crate would handle nicer, but in
         * the same way - we have enough bytes and they're aligned enough to be casted */
        let local_file_header = unsafe { &*local_file_header };
        let packed_file_offset = local_file_offset +
            local_file_header.filename_len as usize +
            local_file_header.extra_field_len as usize;

        let bytes = self.data.get(packed_file_offset..)
            .and_then(|slice| slice.get(..compressed_size))?;

        let file = File {
            compression_method: CompressionMethod::from_u16(central_dir.compression_method)?,
            decompressed_crc: central_dir.decompressed_crc,
            decompressed_size,
            extra_fields,
            filename,
            comment,
            bytes,
        };

        return Some(file);
    }
}

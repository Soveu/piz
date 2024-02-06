use crate::raw;

pub struct Iter<'extra> {
    pub data: &'extra [u8],
}

impl<'extra> Iterator for Iter<'extra> {
    type Item = (u16, &'extra [u8]);
    fn next(&mut self) -> Option<Self::Item> {
        if let [signature_low, signature_high, len_low, len_high, tail @ ..] = self.data {
            let signature = u16::from_le_bytes([*signature_low, *signature_high]);
            let len = u16::from_le_bytes([*len_low, *len_high]) as usize;

            let (extra, tail) = crate::slice_split_at(tail, len)?;
            self.data = tail;
            return Some((signature, extra));
        }

        return None;
    }
}

pub trait Extra: Sized {
    const SIGNATURE: u16;

    fn parse(bytes: &[u8]) -> Option<Self>;
}

pub struct Zip64 {
    pub decompressed_size: u64,
    pub compressed_size: u64,
    pub local_header_record_offset: u64,
    pub disk_no: u32,
}

impl Extra for Zip64 {
    const SIGNATURE: u16 = 0x0001;

    fn parse(bytes: &[u8]) -> Option<Self> {
        let bytes = bytes.get(.. core::mem::size_of::<raw::Zip64Extra>())?;
        let raw = unsafe { &*(bytes.as_ptr() as *const raw::Zip64Extra) };

        Some(Self {
            decompressed_size: raw.decompressed_size,
            compressed_size: raw.compressed_size,
            local_header_record_offset: raw.local_header_record_offset,
            disk_no: raw.disk_no,
        })
    }
}

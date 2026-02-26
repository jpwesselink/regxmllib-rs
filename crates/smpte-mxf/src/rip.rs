use std::io::Cursor;

use smpte_klv::{
    ber::{read_u32_be, read_u64_be},
    MemoryTriplet,
};
use smpte_types::Ul;

use crate::MxfError;

/// Random Index Pack key (SMPTE ST 377-1 §11).
const KEY: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x02, 0x05, 0x01, 0x01,
    0x0D, 0x01, 0x02, 0x01, 0x11, 0x01, 0x00, 0x00,
];

/// One entry in a Random Index Pack.
#[derive(Debug, Clone, Copy)]
pub struct RipEntry {
    pub body_sid: u32,
    pub byte_offset: u64,
}

/// MXF Random Index Pack — enables fast random access to partitions (ST 377-1 §11).
#[derive(Debug, Clone)]
pub struct RandomIndexPack {
    pub entries: Vec<RipEntry>,
}

impl RandomIndexPack {
    /// Parse a RIP from a [`MemoryTriplet`].
    ///
    /// Returns `None` if the triplet key does not match or the length is invalid.
    pub fn from_triplet(triplet: &MemoryTriplet) -> Result<Option<Self>, MxfError> {
        let tpl = Ul::from_bytes(KEY);
        let ul = match triplet.key.as_ul() {
            Some(ul) => ul,
            None => return Ok(None),
        };
        if !ul.equals_with_mask(&tpl, 0xFEFF) {
            return Ok(None);
        }

        let len = triplet.value.len();
        // Last 4 bytes of the value are the overall length field; remaining
        // bytes are 12-byte entries (4 body_sid + 8 offset).
        if len < 4 || (len - 4) % 12 != 0 {
            return Ok(None);
        }

        let count = (len - 4) / 12;
        let mut c = Cursor::new(triplet.value.as_slice());
        let mut entries = Vec::with_capacity(count);

        for _ in 0..count {
            let body_sid    = read_u32_be(&mut c)?;
            let byte_offset = read_u64_be(&mut c)?;
            entries.push(RipEntry { body_sid, byte_offset });
        }

        Ok(Some(RandomIndexPack { entries }))
    }
}

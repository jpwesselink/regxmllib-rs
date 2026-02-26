use std::io::Cursor;

use smpte_klv::{
    ber::{read_u16_be, read_u32_be},
    LocalTagRegister, MemoryTriplet,
};
use smpte_types::Ul;

use crate::MxfError;

/// Primer Pack key (SMPTE ST 377-1 §8.1).
const KEY: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x02, 0x05, 0x01, 0x01,
    0x0D, 0x01, 0x02, 0x01, 0x01, 0x05, 0x01, 0x00,
];

/// MXF Primer Pack — maps 2-byte local tags to ULs (SMPTE ST 377-1 §8.1).
#[derive(Debug, Clone)]
pub struct PrimerPack {
    pub local_tag_register: LocalTagRegister,
}

impl PrimerPack {
    /// Parse a primer pack from a [`MemoryTriplet`].
    ///
    /// Returns `None` if the triplet key does not match the primer pack key.
    pub fn from_triplet(triplet: &MemoryTriplet) -> Result<Option<Self>, MxfError> {
        let tpl = Ul::from_bytes(KEY);
        let ul = match triplet.key.as_ul() {
            Some(ul) => ul,
            None => return Ok(None),
        };
        // Compare all bytes except byte[7] (version).
        if !ul.equals_with_mask(&tpl, 0xFEFF) {
            return Ok(None);
        }

        let mut c = Cursor::new(triplet.value.as_slice());

        let item_count = read_u32_be(&mut c)? as usize;
        let _item_len  = read_u32_be(&mut c)?; // expected to be 18 (2 + 16)

        let mut register = LocalTagRegister::with_capacity(item_count);
        for _ in 0..item_count {
            let local_tag = read_u16_be(&mut c)?;
            let mut ul_bytes = [0u8; 16];
            std::io::Read::read_exact(&mut c, &mut ul_bytes)?;
            register.insert(local_tag, Ul::from_bytes(ul_bytes));
        }

        Ok(Some(PrimerPack { local_tag_register: register }))
    }
}

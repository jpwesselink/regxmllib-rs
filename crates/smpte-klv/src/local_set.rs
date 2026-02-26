use std::collections::HashMap;

use smpte_types::{Auid, Ul};

use crate::{
    ber::read_u16_be,
    KlvError, MemoryTriplet,
};

/// Maps 2-byte local tags to their corresponding [`Ul`] identifiers.
pub type LocalTagRegister = HashMap<u16, Ul>;

/// A KLV local set: a collection of items keyed by resolved [`Auid`]s.
///
/// Local sets use 2-byte tags (resolved via a [`LocalTagRegister`]) rather
/// than full 16-byte keys (SMPTE ST 336 §6.3.3).
#[derive(Debug, Clone)]
pub struct LocalSet {
    pub key: Auid,
    pub items: Vec<MemoryTriplet>,
}

impl LocalSet {
    /// Parse a local set from the value bytes of a KLV triplet, resolving
    /// local tags to ULs using `tag_register`.
    pub fn from_triplet(
        triplet: &MemoryTriplet,
        tag_register: &LocalTagRegister,
    ) -> Result<Self, KlvError> {
        let mut cursor = triplet.value_cursor();
        let total = triplet.value.len() as u64;
        let mut items = Vec::new();

        while cursor.position() < total {
            let local_tag = read_u16_be(&mut cursor)?;
            let local_len = read_u16_be(&mut cursor)? as usize;
            let mut local_val = vec![0u8; local_len];
            std::io::Read::read_exact(&mut cursor, &mut local_val)?;

            let ul = tag_register
                .get(&local_tag)
                .ok_or(KlvError::UnknownLocalTag(local_tag))?;

            items.push(MemoryTriplet {
                key: Auid::Ul(*ul),
                value: local_val,
            });
        }

        Ok(LocalSet {
            key: triplet.key,
            items,
        })
    }
}

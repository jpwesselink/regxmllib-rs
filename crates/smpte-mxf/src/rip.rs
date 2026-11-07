use std::io::Cursor;

use smpte_klv::{
    ber::{read_u32_be, read_u64_be},
    MemoryTriplet,
};
use smpte_types::Ul;

use crate::MxfError;

/// Random Index Pack key (SMPTE ST 377-1:2011 §12.1, Table 29).
///
/// Bytes 12-16 (1-based): Structure Version 01h, Structure Kind 01h,
/// Set/Pack kind 11h (Random Index Pack), Version 01h, Reserved 00h.
const KEY: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x02, 0x05, 0x01, 0x01, 0x0D, 0x01, 0x02, 0x01, 0x01, 0x11, 0x01, 0x00,
];

/// One entry in a Random Index Pack.
#[derive(Debug, Clone, Copy)]
pub struct RipEntry {
    pub body_sid: u32,
    pub byte_offset: u64,
}

/// MXF Random Index Pack — enables fast random access to partitions (ST 377-1:2011 §12).
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
        // Compare all bytes except byte 8 (1-based; registry version), which
        // KLV parsers may ignore (ST 377-1:2011 §6.3, ST 336).
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
            let body_sid = read_u32_be(&mut c)?;
            let byte_offset = read_u64_be(&mut c)?;
            entries.push(RipEntry {
                body_sid,
                byte_offset,
            });
        }

        Ok(Some(RandomIndexPack { entries }))
    }
}

#[cfg(test)]
mod tests {
    use smpte_types::Auid;

    use super::*;

    fn triplet(key: [u8; 16], value: Vec<u8>) -> MemoryTriplet {
        MemoryTriplet::new(Auid::from_bytes(key), value)
    }

    /// One entry (BodySID 1, offset 0x1234) plus the 4-byte overall length.
    fn one_entry_value() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&1u32.to_be_bytes());
        v.extend_from_slice(&0x1234u64.to_be_bytes());
        v.extend_from_slice(&(16u32 + 1 + 16).to_be_bytes());
        v
    }

    #[test]
    fn parses_rip_with_spec_key() {
        // SMPTE ST 377-1:2011 §12.1, Table 29.
        let spec_key = [
            0x06, 0x0E, 0x2B, 0x34, 0x02, 0x05, 0x01, 0x01, 0x0D, 0x01, 0x02, 0x01, 0x01, 0x11,
            0x01, 0x00,
        ];
        let rip = RandomIndexPack::from_triplet(&triplet(spec_key, one_entry_value()))
            .unwrap()
            .expect("spec RIP key must be recognised");
        assert_eq!(rip.entries.len(), 1);
        assert_eq!(rip.entries[0].body_sid, 1);
        assert_eq!(rip.entries[0].byte_offset, 0x1234);
    }

    #[test]
    fn rejects_previous_incorrect_key() {
        // The key this crate used before (bytes 13-16 shifted by one); it is
        // not a RIP key.
        let wrong = [
            0x06, 0x0E, 0x2B, 0x34, 0x02, 0x05, 0x01, 0x01, 0x0D, 0x01, 0x02, 0x01, 0x11, 0x01,
            0x00, 0x00,
        ];
        assert!(
            RandomIndexPack::from_triplet(&triplet(wrong, one_entry_value()))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn ignores_registry_version_byte_only() {
        let mut key = KEY;
        key[7] = 0x02; // byte 8: registry version may vary
        assert!(
            RandomIndexPack::from_triplet(&triplet(key, one_entry_value()))
                .unwrap()
                .is_some()
        );

        // Any other byte differing must not match (e.g. byte 15, pack version).
        let mut key = KEY;
        key[14] = 0x02;
        assert!(
            RandomIndexPack::from_triplet(&triplet(key, one_entry_value()))
                .unwrap()
                .is_none()
        );
    }
}

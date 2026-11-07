use smpte_klv::MemoryTriplet;
use smpte_types::Ul;

/// KLV Fill Item key as registered in SMPTE RP 210 (quoted in ST 377-1:2011
/// §6.3.3). Byte 8 (1-based; version) is `02h`; some early encoders wrote
/// `01h`, so decoders shall ignore that byte.
const FILL_KEY: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x01, 0x01, 0x02, 0x03, 0x01, 0x02, 0x10, 0x01, 0x00, 0x00, 0x00,
];

/// Returns `true` if `triplet` is a KLV fill item.
pub fn is_fill_item(triplet: &MemoryTriplet) -> bool {
    if let Some(ul) = triplet.key.as_ul() {
        let fill = Ul::from_bytes(FILL_KEY);
        ul.equals_ignore_version(&fill)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use smpte_types::Auid;

    use super::*;

    fn fill(key: [u8; 16]) -> bool {
        is_fill_item(&MemoryTriplet::new(Auid::from_bytes(key), vec![]))
    }

    #[test]
    fn recognises_both_fill_key_versions() {
        // ST 377-1:2011 §6.3.3: RP 210 key (version 02h) and the legacy
        // version-01h key; decoders shall ignore byte 8.
        let mut key = [
            0x06, 0x0E, 0x2B, 0x34, 0x01, 0x01, 0x01, 0x02, 0x03, 0x01, 0x02, 0x10, 0x01, 0x00,
            0x00, 0x00,
        ];
        assert!(fill(key));
        key[7] = 0x01;
        assert!(fill(key));
    }

    #[test]
    fn rejects_key_differing_outside_version_byte() {
        let mut key = FILL_KEY;
        key[8] = 0x0D; // byte 9 (1-based): item designator must match
        assert!(!fill(key));
    }
}

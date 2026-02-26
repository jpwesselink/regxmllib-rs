use smpte_klv::MemoryTriplet;
use smpte_types::Ul;

/// KLV Fill Item key (SMPTE ST 336 §6.3.2).
const FILL_KEY: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x01, 0x01, 0x01,
    0x03, 0x01, 0x02, 0x10, 0x01, 0x00, 0x00, 0x00,
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

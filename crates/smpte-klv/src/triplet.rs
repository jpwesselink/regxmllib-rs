use smpte_types::Auid;

/// A KLV triplet whose value bytes are held in heap memory.
#[derive(Debug, Clone)]
pub struct MemoryTriplet {
    pub key: Auid,
    pub value: Vec<u8>,
}

impl MemoryTriplet {
    pub fn new(key: Auid, value: Vec<u8>) -> Self {
        MemoryTriplet { key, value }
    }

    pub fn value_cursor(&self) -> std::io::Cursor<&[u8]> {
        std::io::Cursor::new(&self.value)
    }
}

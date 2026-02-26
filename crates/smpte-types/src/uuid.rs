/// SMPTE UUID wrapper that encapsulates MXF byte-swap semantics.
///
/// In MXF, UUIDs are stored with bytes 0-7 swapped with bytes 8-15 relative
/// to the RFC 4122 layout. This wrapper centralises that conversion.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Uuid(uuid::Uuid);

impl Uuid {
    /// Construct from raw MXF bytes (applying the MXF byte-swap).
    pub fn from_bytes_mxf(b: [u8; 16]) -> Self {
        let mut swapped = [0u8; 16];
        swapped[..8].copy_from_slice(&b[8..]);
        swapped[8..].copy_from_slice(&b[..8]);
        Uuid(uuid::Uuid::from_bytes(swapped))
    }

    /// Construct directly from RFC 4122 bytes (no swap).
    pub fn from_bytes(b: [u8; 16]) -> Self {
        Uuid(uuid::Uuid::from_bytes(b))
    }

    pub fn as_inner(&self) -> &uuid::Uuid {
        &self.0
    }
}

impl std::fmt::Display for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Debug for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Uuid({})", self.0)
    }
}

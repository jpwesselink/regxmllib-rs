use thiserror::Error;

/// SMPTE Unique Material Identifier — a 32-byte identifier (SMPTE ST 330).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Umid([u8; 32]);

#[derive(Debug, Error)]
pub enum UmidParseError {
    #[error("invalid UMID URN prefix, expected 'urn:smpte:umid:'")]
    InvalidPrefix,
    #[error("wrong number of hex bytes: expected 32, got {0}")]
    WrongLength(usize),
    #[error("invalid hex digit at byte position {0}")]
    InvalidHex(usize),
}

impl Umid {
    pub fn from_bytes(b: [u8; 32]) -> Self {
        Umid(b)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for Umid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "urn:smpte:umid:")?;
        for (i, b) in self.0.iter().enumerate() {
            if i > 0 && i % 4 == 0 {
                write!(f, ".")?;
            }
            write!(f, "{b:02X}")?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for Umid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Umid({})", self)
    }
}

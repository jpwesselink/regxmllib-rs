use thiserror::Error;

/// SMPTE Universal Label — a structured 16-byte identifier (SMPTE ST 298).
///
/// The bytes encode: designator (0x06 0x0E 0x2B 0x34), category, registry,
/// structure, version, and item designation.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ul([u8; 16]);

#[derive(Debug, Error)]
pub enum UlParseError {
    #[error("invalid URN prefix, expected 'urn:smpte:ul:'")]
    InvalidPrefix,
    #[error("wrong number of hex bytes: expected 16, got {0}")]
    WrongLength(usize),
    #[error("invalid hex digit at byte position {0}")]
    InvalidHex(usize),
}

impl Ul {
    pub fn from_bytes(b: [u8; 16]) -> Self {
        Ul(b)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Parse from `urn:smpte:ul:AABBCCDD.AABBCCDD.AABBCCDD.AABBCCDD` format.
    pub fn from_urn(s: &str) -> Result<Self, UlParseError> {
        let hex = s
            .strip_prefix("urn:smpte:ul:")
            .ok_or(UlParseError::InvalidPrefix)?;
        let hex = hex.replace('.', "");
        if hex.len() != 32 {
            return Err(UlParseError::WrongLength(hex.len() / 2));
        }
        let mut bytes = [0u8; 16];
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
            let hi = hex_nibble(chunk[0]).map_err(|_| UlParseError::InvalidHex(i))?;
            let lo = hex_nibble(chunk[1]).map_err(|_| UlParseError::InvalidHex(i))?;
            bytes[i] = (hi << 4) | lo;
        }
        Ok(Ul(bytes))
    }

    /// Compare with a 16-bit bitmask: bit N (0 = MSB of byte 0) enables byte N/2.
    /// Used for version-agnostic comparison where bit 8 (byte 7) is the version byte.
    pub fn equals_with_mask(&self, other: &Ul, mask: u16) -> bool {
        for i in 0..16usize {
            let bit = 1u16 << (15 - i);
            if mask & bit != 0 && self.0[i] != other.0[i] {
                return false;
            }
        }
        true
    }

    /// Compare ignoring byte 7 (version byte).
    pub fn equals_ignore_version(&self, other: &Ul) -> bool {
        self.equals_with_mask(other, 0xFF7F)
    }

    pub fn is_group_key(&self) -> bool {
        self.0[4] == 0x02
    }
}

fn hex_nibble(b: u8) -> Result<u8, ()> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(()),
    }
}

impl std::fmt::Display for Ul {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "urn:smpte:ul:{:02x}{:02x}{:02x}{:02x}.{:02x}{:02x}{:02x}{:02x}.{:02x}{:02x}{:02x}{:02x}.{:02x}{:02x}{:02x}{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3],
            self.0[4], self.0[5], self.0[6], self.0[7],
            self.0[8], self.0[9], self.0[10], self.0[11],
            self.0[12], self.0[13], self.0[14], self.0[15],
        )
    }
}

impl std::fmt::Debug for Ul {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ul({})", self)
    }
}

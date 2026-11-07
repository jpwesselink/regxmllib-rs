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

    /// Compare with a 16-bit byte mask, MSB-first: bit `15 - i` of `mask`
    /// enables comparison of byte `i` (0-based). `0x8000` selects byte 0,
    /// `0x0001` selects byte 15. For example `0xFEFF` compares every byte
    /// except byte 7 (the version byte, "byte 8" in SMPTE 1-based numbering).
    pub fn equals_with_mask(&self, other: &Ul, mask: u16) -> bool {
        for i in 0..16usize {
            let bit = 1u16 << (15 - i);
            if mask & bit != 0 && self.0[i] != other.0[i] {
                return false;
            }
        }
        true
    }

    /// Compare ignoring the version byte: byte 7 0-based, "byte 8" in SMPTE
    /// 1-based numbering (SMPTE ST 336; ST 377-1:2011 §6.3).
    pub fn equals_ignore_version(&self, other: &Ul) -> bool {
        self.equals_with_mask(other, 0xFEFF)
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

#[cfg(test)]
mod tests {
    use super::*;

    const FILL_V2: [u8; 16] = [
        0x06, 0x0E, 0x2B, 0x34, 0x01, 0x01, 0x01, 0x02, 0x03, 0x01, 0x02, 0x10, 0x01, 0x00, 0x00,
        0x00,
    ];

    #[test]
    fn mask_bit_order_is_msb_first() {
        let a = Ul::from_bytes([0; 16]);
        for i in 0..16 {
            let mut b = [0u8; 16];
            b[i] = 1;
            let b = Ul::from_bytes(b);
            assert!(
                !a.equals_with_mask(&b, 0x8000 >> i),
                "byte {i} must be compared"
            );
            assert!(
                a.equals_with_mask(&b, !(0x8000u16 >> i)),
                "byte {i} must be skipped"
            );
        }
    }

    #[test]
    fn ignore_version_skips_version_byte_only() {
        let v2 = Ul::from_bytes(FILL_V2);
        let mut v1 = FILL_V2;
        v1[7] = 0x01;
        assert!(v2.equals_ignore_version(&Ul::from_bytes(v1)));

        // Byte 8 (0-based; item designator) is not the version byte.
        let mut other = FILL_V2;
        other[8] = 0x0D;
        assert!(!v2.equals_ignore_version(&Ul::from_bytes(other)));
    }
}

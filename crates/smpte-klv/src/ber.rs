use std::io::Read;

use crate::KlvError;

/// Read a BER-encoded length from `r` (SMPTE ST 336 §6.3).
///
/// Short form: 1 byte, value 0x00–0x7F.
/// Long form:  first byte = 0x80 | N, followed by N big-endian length bytes (N ≤ 8).
pub fn read_ber_length(r: &mut impl Read) -> Result<u64, KlvError> {
    let first = read_u8(r)?;
    if first & 0x80 == 0 {
        return Ok(first as u64);
    }
    let count = (first & 0x7F) as usize;
    if count == 0 || count > 8 {
        return Err(KlvError::BerLengthTooLong(count));
    }
    let mut val = 0u64;
    for _ in 0..count {
        val = (val << 8) | (read_u8(r)? as u64);
    }
    Ok(val)
}

/// Number of bytes a BER encoding of `length` will occupy.
pub fn ber_encoded_len(length: u64) -> u64 {
    if length <= 0x7F {
        1
    } else if length <= 0xFF {
        2
    } else if length <= 0xFFFF {
        3
    } else if length <= 0xFFFFFF {
        4
    } else if length <= 0xFFFFFFFF {
        5
    } else {
        9 // worst case 8-byte long form
    }
}

pub(crate) fn read_u8(r: &mut impl Read) -> Result<u8, KlvError> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

pub fn read_u16_be(r: &mut impl Read) -> Result<u16, KlvError> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
}

pub fn read_u32_be(r: &mut impl Read) -> Result<u32, KlvError> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

pub fn read_u64_be(r: &mut impl Read) -> Result<u64, KlvError> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_be_bytes(buf))
}

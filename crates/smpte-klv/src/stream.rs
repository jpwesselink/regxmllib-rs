use std::io::{Read, Seek};

use smpte_types::Auid;

use crate::{
    ber::{read_ber_length, ber_encoded_len},
    KlvError, MemoryTriplet,
};

const DEFAULT_MAX_VALUE_LEN: u64 = 256 * 1024 * 1024; // 256 MiB

/// Sequential KLV triplet reader over any `Read + Seek` source.
pub struct KlvStream<R: Read + Seek> {
    inner: R,
    position: u64,
    max_value_len: u64,
}

impl<R: Read + Seek> KlvStream<R> {
    pub fn new(r: R) -> Self {
        KlvStream {
            inner: r,
            position: 0,
            max_value_len: DEFAULT_MAX_VALUE_LEN,
        }
    }

    pub fn with_max_value_len(mut self, max: u64) -> Self {
        self.max_value_len = max;
        self
    }

    pub fn position(&self) -> u64 {
        self.position
    }

    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Read the next KLV triplet, or return `None` at end-of-stream.
    pub fn read_triplet(&mut self) -> Result<Option<MemoryTriplet>, KlvError> {
        let mut key_bytes = [0u8; 16];
        match self.inner.read(&mut key_bytes[..1]) {
            Ok(0) => return Ok(None),
            Ok(_) => {}
            Err(e) => return Err(KlvError::Io(e)),
        }
        self.inner
            .read_exact(&mut key_bytes[1..])
            .map_err(|_| KlvError::UnexpectedEof)?;

        let key = Auid::from_bytes(key_bytes);
        let length = read_ber_length(&mut self.inner)?;

        if length > self.max_value_len {
            return Err(KlvError::ValueTooLarge(length, self.max_value_len));
        }

        let mut value = vec![0u8; length as usize];
        self.inner.read_exact(&mut value)?;

        self.position += 16 + ber_encoded_len(length) + length;

        Ok(Some(MemoryTriplet { key, value }))
    }
}

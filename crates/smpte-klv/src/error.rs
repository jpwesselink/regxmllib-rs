use thiserror::Error;

#[derive(Debug, Error)]
pub enum KlvError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("BER long-form length uses {0} octets; maximum is 8")]
    BerLengthTooLong(usize),
    #[error("value length {0} bytes exceeds configured maximum {1}")]
    ValueTooLarge(u64, u64),
    #[error("unknown local tag 0x{0:04X}")]
    UnknownLocalTag(u16),
    #[error("unexpected end of stream reading KLV key")]
    UnexpectedEof,
}

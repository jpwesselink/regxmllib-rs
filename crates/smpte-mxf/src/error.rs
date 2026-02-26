use thiserror::Error;

use smpte_klv::KlvError;

#[derive(Debug, Error)]
pub enum MxfError {
    #[error("KLV error: {0}")]
    Klv(#[from] KlvError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("no partition pack found within run-in (first 65536 bytes)")]
    PartitionNotFound,
    #[error("invalid or unexpected partition pack key")]
    InvalidPartitionKey,
    #[error("truncated MXF: {0}")]
    Truncated(&'static str),
    #[error("unsupported MXF version {0}.{1}")]
    UnsupportedVersion(u16, u16),
    #[error("missing primer pack in header partition")]
    MissingPrimerPack,
}

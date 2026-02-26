use crate::{Ul, Uuid};
use thiserror::Error;

/// SMPTE AUID — either a Universal Label or a UUID (SMPTE ST 2001-1 §7).
///
/// Bit 7 of byte 0 distinguishes the two: `0` → [`Ul`], `1` → [`Uuid`].
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Auid {
    Ul(Ul),
    Uuid(Uuid),
}

#[derive(Debug, Error)]
pub enum AuidParseError {
    #[error("invalid AUID URN — expected 'urn:smpte:ul:' or 'urn:uuid:' prefix, got: {0:?}")]
    InvalidPrefix(String),
    #[error("UL parse error: {0}")]
    Ul(#[from] crate::ul::UlParseError),
    #[error("UUID parse error: {0}")]
    Uuid(String),
}

impl Auid {
    /// Construct from 16 raw bytes, choosing UL or UUID by bit 7 of byte 0.
    pub fn from_bytes(b: [u8; 16]) -> Self {
        if b[0] & 0x80 == 0 {
            Auid::Ul(Ul::from_bytes(b))
        } else {
            Auid::Uuid(Uuid::from_bytes_mxf(b))
        }
    }

    /// Parse from `urn:smpte:ul:…` or `urn:uuid:…` format.
    pub fn from_urn(s: &str) -> Result<Self, AuidParseError> {
        if s.starts_with("urn:smpte:ul:") {
            Ok(Auid::Ul(Ul::from_urn(s)?))
        } else if let Some(inner) = s.strip_prefix("urn:uuid:") {
            let u = uuid::Uuid::parse_str(inner)
                .map_err(|e| AuidParseError::Uuid(e.to_string()))?;
            Ok(Auid::Uuid(Uuid::from_bytes(*u.as_bytes())))
        } else {
            Err(AuidParseError::InvalidPrefix(s.chars().take(32).collect()))
        }
    }

    pub fn as_ul(&self) -> Option<&Ul> {
        match self {
            Auid::Ul(ul) => Some(ul),
            _ => None,
        }
    }

    pub fn as_uuid(&self) -> Option<&Uuid> {
        match self {
            Auid::Uuid(uuid) => Some(uuid),
            _ => None,
        }
    }

    pub fn is_ul(&self) -> bool {
        matches!(self, Auid::Ul(_))
    }

    pub fn is_uuid(&self) -> bool {
        matches!(self, Auid::Uuid(_))
    }
}

impl std::fmt::Display for Auid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Auid::Ul(ul) => write!(f, "{ul}"),
            Auid::Uuid(uuid) => write!(f, "urn:uuid:{uuid}"),
        }
    }
}

impl std::fmt::Debug for Auid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Auid::Ul(ul) => write!(f, "Auid::Ul({ul:?})"),
            Auid::Uuid(uuid) => write!(f, "Auid::Uuid({uuid:?})"),
        }
    }
}

use thiserror::Error;

use smpte_types::Auid;

#[derive(Debug, Error)]
pub enum FragmentError {
    #[error("unknown group: {0}")]
    UnknownGroup(Auid),
    #[error("unknown property: {0}")]
    UnknownProperty(Auid),
    #[error("MXF error: {0}")]
    Mxf(#[from] smpte_mxf::MxfError),
    #[error("XML write error: {0}")]
    Xml(String),
    #[error("fatal event: {0}")]
    Fatal(String),
    #[error("unsupported type variant: {0}")]
    UnsupportedType(String),
}

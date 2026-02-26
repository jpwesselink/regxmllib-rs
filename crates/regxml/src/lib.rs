//! RegXML fragment builder and XML schema generator (SMPTE ST 2001-1).
//!
//! Provides [`FragmentBuilder`] (converts MXF header metadata sets to RegXML),
//! [`MxfFragmentBuilder`] (orchestrates full MXF-to-RegXML pipeline), and
//! [`XmlSchemaBuilder`] (derives XSD from a metadictionary).

pub mod error;
pub mod event;
pub mod fragment;
pub mod mxf_fragment;
pub mod schema;

pub use error::FragmentError;
pub use event::{EventCode, EventHandler, EventSeverity, FragmentEvent};
pub use fragment::{AuidNamer, FragmentBuilder};
pub use mxf_fragment::{
    MxfFragmentBuilder, MxfFragmentError, MxfFragmentOptions, PartitionTarget, RootMode,
};
pub use schema::XmlSchemaBuilder;

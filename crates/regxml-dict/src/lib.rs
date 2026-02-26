//! RegXML metadictionary and definition types (SMPTE ST 2001-1).
//!
//! Provides [`Definition`], [`MetaDictionary`], [`MetaDictionaryCollection`],
//! the [`DefinitionResolver`] trait, and the SMPTE register XML importer.

pub mod definition;
pub mod error;
pub mod importer;
pub mod labels;
pub mod meta_dictionary;
pub mod resolver;

pub use definition::{ClassDefinition, Definition, PropertyDefinition, TypeDefinition};
pub use error::DictError;
pub use labels::LabelsRegister;
pub use meta_dictionary::{MetaDictionary, MetaDictionaryCollection};
pub use resolver::DefinitionResolver;

use smpte_types::Auid;

use crate::definition::{ClassDefinition, Definition};

/// Trait for looking up SMPTE metadata definitions by AUID (ST 2001-1 §7).
pub trait DefinitionResolver {
    fn get_definition(&self, id: &Auid) -> Option<&Definition>;
    fn get_subclasses_of(&self, class: &ClassDefinition) -> Vec<Auid>;
    fn get_members_of(&self, class: &ClassDefinition) -> Vec<Auid>;
}

use crate::core::BindingId;
use crate::types::{DefinitionTable, TypedDefinition};

pub trait DefinitionLookup {
    fn definition(&self, binding: BindingId) -> Option<&TypedDefinition>;
}

impl DefinitionLookup for DefinitionTable<'_> {
    fn definition(&self, binding: BindingId) -> Option<&TypedDefinition> {
        DefinitionTable::definition(self, binding)
    }
}

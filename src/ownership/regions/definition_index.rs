use rustc_hash::FxHashMap;

use super::definition_lookup::DefinitionLookup;
use crate::core::BindingId;
use crate::types::{TypedDefinition, TypedProgram};

pub struct DefinitionIndex<'p> {
    definitions: FxHashMap<BindingId, &'p TypedDefinition>,
}

impl<'p> DefinitionIndex<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        Self {
            definitions: program
                .modules
                .iter()
                .flat_map(|module| &module.definitions)
                .map(|definition| (definition.binding, definition))
                .collect(),
        }
    }
}

impl DefinitionLookup for DefinitionIndex<'_> {
    fn definition(&self, binding: BindingId) -> Option<&TypedDefinition> {
        self.definitions.get(&binding).copied()
    }
}

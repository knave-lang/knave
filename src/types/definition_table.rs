use rustc_hash::FxHashMap;

use super::definition_entry::DefinitionEntry;
use super::typed::{TypedDefinition, TypedProgram};
use crate::core::BindingId;

pub struct DefinitionTable<'p> {
    entries: FxHashMap<BindingId, DefinitionEntry<'p>>,
}

impl<'p> DefinitionTable<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        Self {
            entries: program
                .modules
                .iter()
                .flat_map(|module| {
                    module.definitions.iter().map(move |definition| {
                        let entry = DefinitionEntry {
                            definition,
                            module: module.id,
                        };
                        (definition.binding, entry)
                    })
                })
                .collect(),
        }
    }

    pub fn entry(&self, binding: BindingId) -> Option<DefinitionEntry<'p>> {
        self.entries.get(&binding).copied()
    }

    pub fn definition(&self, binding: BindingId) -> Option<&'p TypedDefinition> {
        self.entry(binding).map(|entry| entry.definition)
    }
}

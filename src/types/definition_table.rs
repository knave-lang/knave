use rustc_hash::FxHashMap;

use super::typed::{TypedDefinition, TypedProgram};
use crate::core::{BindingId, ModuleId};

pub struct DefinitionTable<'p> {
    entries: FxHashMap<BindingId, (&'p TypedDefinition, ModuleId)>,
}

impl<'p> DefinitionTable<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        Self {
            entries: program
                .modules
                .iter()
                .flat_map(|module| {
                    module
                        .definitions
                        .iter()
                        .map(move |definition| (definition.binding, (definition, module.id)))
                })
                .collect(),
        }
    }

    pub fn entry(&self, binding: BindingId) -> Option<(&'p TypedDefinition, ModuleId)> {
        self.entries.get(&binding).copied()
    }

    pub fn definition(&self, binding: BindingId) -> Option<&'p TypedDefinition> {
        self.entry(binding).map(|(definition, _)| definition)
    }
}

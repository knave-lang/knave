use rustc_hash::FxHashMap;

use super::text_names::TextNames;
use super::text_traits::TextTraits;
use super::text_types::TextTypes;
use crate::core::BindingId;
use crate::modules::ModuleSources;
use crate::types::{TypedDefinition, TypedModule, TypedProgram};

pub struct TextProgram<'p> {
    program: &'p TypedProgram,
    definitions: FxHashMap<BindingId, &'p TypedDefinition>,
}

impl<'p> TextProgram<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        let definitions = program
            .modules
            .iter()
            .flat_map(|module| &module.definitions)
            .map(|definition| (definition.binding, definition))
            .collect();
        Self {
            program,
            definitions,
        }
    }

    pub fn written_modules(&self) -> impl Iterator<Item = &'p TypedModule> + '_ {
        self.program
            .modules
            .iter()
            .filter(|module| !ModuleSources::is_prelude(&module.path))
    }

    pub fn definition(&self, binding: BindingId) -> Option<&'p TypedDefinition> {
        self.definitions.get(&binding).copied()
    }

    pub fn traits(&self) -> TextTraits<'p> {
        TextTraits::new(self.program.declarations.traits().table())
    }

    pub fn names(&self) -> TextNames<'p> {
        TextNames::new(self.program)
    }

    pub fn types(&self) -> TextTypes<'p> {
        TextTypes::new(self.program)
    }
}

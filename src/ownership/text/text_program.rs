use rustc_hash::{FxHashMap, FxHashSet};

use super::text_names::TextNames;
use super::text_traits::TextTraits;
use super::text_types::TextTypes;
use crate::core::BindingId;
use crate::modules::ModuleSources;
use crate::types::{Type, TypedDefinition, TypedModule, TypedProgram};

pub struct TextProgram<'p> {
    program: &'p TypedProgram,
    definitions: FxHashMap<BindingId, &'p TypedDefinition>,
    written_values: FxHashSet<BindingId>,
}

impl<'p> TextProgram<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        let definitions = program
            .modules
            .iter()
            .flat_map(|module| &module.definitions)
            .map(|definition| (definition.binding, definition))
            .collect();
        let written_values = program
            .modules
            .iter()
            .filter(|module| !ModuleSources::is_prelude(&module.path))
            .flat_map(|module| &module.definitions)
            .filter(|definition| !matches!(definition.value.ty, Type::Function(..)))
            .map(|definition| definition.binding)
            .collect();
        Self {
            program,
            definitions,
            written_values,
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

    pub fn is_written_value(&self, binding: BindingId) -> bool {
        self.written_values.contains(&binding)
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

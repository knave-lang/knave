use super::read_back_definition::ReadBackDefinition;
use crate::core::{BindingId, CoreModule};

pub struct LintedModule<'p> {
    module: &'p CoreModule,
    definitions: Vec<ReadBackDefinition>,
}

impl<'p> LintedModule<'p> {
    pub fn new(module: &'p CoreModule, definitions: Vec<ReadBackDefinition>) -> Self {
        Self {
            module,
            definitions,
        }
    }

    pub fn module(&self) -> &'p CoreModule {
        self.module
    }

    pub fn reads(&self) -> impl Iterator<Item = BindingId> + '_ {
        self.definitions
            .iter()
            .flat_map(|definition| definition.table.references())
    }

    pub fn into_definitions(self) -> Vec<ReadBackDefinition> {
        self.definitions
    }
}

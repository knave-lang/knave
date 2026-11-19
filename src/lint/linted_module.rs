use super::read_back_definition::ReadBackDefinition;
use crate::core::CoreModule;

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

    pub fn into_definitions(self) -> Vec<ReadBackDefinition> {
        self.definitions
    }
}

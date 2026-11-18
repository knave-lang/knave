use super::typed::{TypedDefinition, TypedProgram};
use crate::core::BindingId;

pub struct ProgramDefinitions<'p> {
    program: &'p TypedProgram,
}

impl<'p> ProgramDefinitions<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub fn find(&self, binding: BindingId) -> Option<&'p TypedDefinition> {
        self.program
            .modules
            .iter()
            .flat_map(|module| &module.definitions)
            .find(|definition| definition.binding == binding)
    }
}

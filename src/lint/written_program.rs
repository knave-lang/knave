use crate::core::{BindingId, CoreModule, Item};
use crate::expand::Program;

pub struct WrittenProgram<'p> {
    program: &'p Program,
}

impl<'p> WrittenProgram<'p> {
    pub fn new(program: &'p Program) -> Self {
        Self { program }
    }

    pub fn is_written(&self, item: &Item) -> bool {
        self.program
            .call_origins
            .introducing_macro(item.span)
            .is_none()
    }

    pub fn library_modules(&self) -> impl Iterator<Item = &'p CoreModule> {
        let entry = self.program.entry.module;
        self.program
            .project_modules()
            .filter(move |module| module.id != entry)
    }

    pub fn name(&self, binding: BindingId) -> String {
        self.program.bindings.get(binding).name().to_string()
    }
}

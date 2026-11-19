use crate::core::{BinderOrigin, BindingId};
use crate::types::TypedProgram;

#[derive(Clone, Copy)]
pub struct TextNames<'p> {
    program: &'p TypedProgram,
}

impl<'p> TextNames<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub fn is_generated(&self, binding: BindingId) -> bool {
        self.program.synthetic_bindings.contains(&binding)
            || self.program.bindings.origin(binding) != BinderOrigin::Written
    }

    pub fn name(&self, binding: BindingId) -> &'p str {
        self.program.bindings.get(binding).name()
    }
}

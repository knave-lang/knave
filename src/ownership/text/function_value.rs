use super::call_target::CallTarget;
use super::callee_scheme::CalleeScheme;
use super::slot::Slot;
use super::type_path::TypePath;

pub struct FunctionValue<'t> {
    scheme: CalleeScheme<'t>,
    value: Slot,
}

impl<'t> FunctionValue<'t> {
    pub fn new(scheme: CalleeScheme<'t>, value: Slot) -> Self {
        Self { scheme, value }
    }
}

impl<'t> CallTarget<'t> for FunctionValue<'t> {
    fn scheme(&self) -> &CalleeScheme<'t> {
        &self.scheme
    }

    fn parameter(&self, index: usize) -> Option<Slot> {
        Some(self.value.nested(&TypePath::default().child(index)))
    }

    fn result(&self) -> Option<Slot> {
        Some(self.value.nested(&TypePath::default().result()))
    }
}

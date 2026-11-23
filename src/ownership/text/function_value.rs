use super::call_target::CallTarget;
use super::callee_scheme::CalleeScheme;
use super::function_slots::FunctionSlots;
use super::slot::Slot;

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
        Some(self.value.nested(&FunctionSlots::parameter_path(index)))
    }

    fn result(&self) -> Option<Slot> {
        Some(self.value.nested(&FunctionSlots::result_path()))
    }
}

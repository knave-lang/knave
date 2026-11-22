use super::call_target::CallTarget;
use super::callee_scheme::CalleeScheme;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use crate::core::BindingId;
use crate::types::TypedLambda;

pub struct UserFunction<'t> {
    scheme: CalleeScheme<'t>,
    binding: BindingId,
    parameters: Vec<BindingId>,
}

impl<'t> UserFunction<'t> {
    pub fn new(scheme: CalleeScheme<'t>, binding: BindingId, lambda: &TypedLambda) -> Self {
        Self {
            scheme,
            binding,
            parameters: lambda
                .params
                .iter()
                .map(|parameter| parameter.binding)
                .collect(),
        }
    }
}

impl<'t> CallTarget<'t> for UserFunction<'t> {
    fn scheme(&self) -> &CalleeScheme<'t> {
        &self.scheme
    }

    fn parameter(&self, index: usize) -> Option<Slot> {
        self.parameters
            .get(index)
            .map(|binding| Slot::from(SlotOwner::Parameter(*binding)))
    }

    fn result(&self) -> Option<Slot> {
        Some(Slot::from(SlotOwner::Result(self.binding)))
    }
}

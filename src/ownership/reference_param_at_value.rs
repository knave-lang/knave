use crate::types::{Borrow, Type, TypedArgument};

pub(super) struct ReferenceParamAtValue<'t> {
    param: &'t Type,
    instantiated: Option<&'t Type>,
}

impl<'t> ReferenceParamAtValue<'t> {
    pub(super) fn new(param: &'t Type, instantiated: Option<&'t Type>) -> Self {
        Self {
            param,
            instantiated,
        }
    }

    pub(super) fn keep_one_reference(&self, argument: &mut TypedArgument) {
        if !self.is_instantiated_at_a_value() {
            return;
        }
        argument.borrow = match argument.borrow {
            Borrow::Deref(1) => Borrow::Passthrough,
            Borrow::Deref(levels) => Borrow::Deref(levels - 1),
            kept => kept,
        };
    }

    fn is_instantiated_at_a_value(&self) -> bool {
        let instantiated_as_value = self.instantiated.is_some_and(|value| !value.is_reference());
        self.param.is_reference() && instantiated_as_value
    }
}

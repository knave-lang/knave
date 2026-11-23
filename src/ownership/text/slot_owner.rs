use std::rc::Rc;

use crate::core::BindingId;
use crate::source::Span;
use crate::types::GenericId;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SlotOwner {
    Parameter(BindingId),
    Result(BindingId),
    Local(BindingId),
    Field(BindingId, Rc<str>),
    Global(BindingId),
    ClosureResult(Span),
    TraitMethod(BindingId),
    Variable(Span, GenericId),
    Value(Span),
    Argument(Span),
    Written(Span),
}

impl SlotOwner {
    pub fn is_variable(&self) -> bool {
        matches!(self, Self::Variable(..))
    }

    pub fn is_decided(&self) -> bool {
        matches!(
            self,
            Self::Parameter(_)
                | Self::Result(_)
                | Self::Local(_)
                | Self::Field(..)
                | Self::Global(_)
                | Self::ClosureResult(_)
        )
    }
}

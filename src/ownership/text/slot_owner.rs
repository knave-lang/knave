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
    Variable(Span, GenericId),
    Value(Span),
    Argument(Span),
    Written(Span),
}

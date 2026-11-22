use super::callee_scheme::CalleeScheme;
use super::representation::Representation;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::text_value::TextValue;
use super::walk_context::WalkContext;
use crate::source::Span;
use crate::types::TypedExpr;

pub trait CallTarget<'t> {
    fn scheme(&self) -> &CalleeScheme<'t>;

    fn inline_value(
        &self,
        _context: &mut WalkContext<'_, '_>,
        _call: &TypedExpr,
        _values: &[TextValue],
    ) -> Option<TextValue> {
        None
    }

    fn converted(&self, _values: &[TextValue]) -> Option<TextValue> {
        None
    }

    fn parameter(&self, _index: usize) -> Option<Slot> {
        None
    }

    fn owned_argument(&self, _call: Span) -> Option<SlotOwner> {
        None
    }

    fn changes_in_place(&self) -> bool {
        false
    }

    fn result(&self) -> Option<Slot> {
        None
    }

    fn made(
        &self,
        _written: Representation,
        _arguments: &[TextValue],
        _call: Span,
    ) -> Option<TextValue> {
        None
    }
}

use super::need::Need;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::walk_context::WalkContext;
use crate::types::TypedBinder;

pub struct ParameterPins<'c, 'w, 'p> {
    context: &'c mut WalkContext<'w, 'p>,
}

impl<'c, 'w, 'p> ParameterPins<'c, 'w, 'p> {
    pub fn new(context: &'c mut WalkContext<'w, 'p>) -> Self {
        Self { context }
    }

    pub fn pin_written(&mut self, owner: SlotOwner, binder: &TypedBinder) {
        if binder.written_view {
            self.pin(owner);
        }
    }

    pub fn pin_declared(&mut self, owner: SlotOwner, binder: &TypedBinder) {
        if TypedBinder::signature_writes_a_view(&binder.ty) {
            self.pin(owner);
        }
    }

    fn pin(&mut self, owner: SlotOwner) {
        self.context.require(Slot::from(owner), Need::View);
    }
}

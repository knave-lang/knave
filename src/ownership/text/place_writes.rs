use super::need::Need;
use super::node_id::NodeId;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_value::TextValue;
use super::walk_context::WalkContext;
use crate::types::TypedExpr;

pub struct PlaceWrites<'c, 'w, 'p> {
    context: &'c mut WalkContext<'w, 'p>,
    call: &'c TypedExpr,
}

impl<'c, 'w, 'p> PlaceWrites<'c, 'w, 'p> {
    pub fn new(context: &'c mut WalkContext<'w, 'p>, call: &'c TypedExpr) -> Self {
        Self { context, call }
    }

    pub fn through_variable(mut self, variable: Slot, place: &TextValue) {
        let written = self.written();
        let changed = self.context.node(variable.clone());
        self.context.unite(written, changed);
        self.unite_with_place(changed, place);
        for (path, nested) in place.nested() {
            let inner = self.context.node(variable.nested(path));
            self.context.unite(inner, *nested);
        }
    }

    pub fn in_place(mut self, place: &TextValue) {
        let changed = Slot::from(SlotOwner::Written(self.call.span));
        self.context.require(changed, Need::Owned);
        let written = self.written();
        self.unite_with_place(written, place);
    }

    fn written(&mut self) -> NodeId {
        self.context
            .node(Slot::from(SlotOwner::Written(self.call.span)))
    }

    fn unite_with_place(&mut self, node: NodeId, place: &TextValue) {
        for (source, _, _) in place.sources() {
            if let Source::Slot(slot) = source {
                self.context.unite(node, *slot);
            }
        }
    }
}

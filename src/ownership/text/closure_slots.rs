use super::function_slots::FunctionSlots;
use super::need::Need;
use super::node_id::NodeId;
use super::slot::Slot;
use super::slot_listing::SlotListing;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_position::TextPosition;
use super::text_value::TextValue;
use super::transfer::Transfer;
use super::type_path::TypePath;
use super::walk_context::WalkContext;
use crate::core::BindingId;
use crate::types::{Type, TypedBinder, TypedExpr, TypedLambda};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ClosureUse {
    Free,
    Fixed,
}

pub struct ClosureSlots<'l> {
    lambda: &'l TypedLambda,
    closure: &'l TypedExpr,
    closure_use: ClosureUse,
}

impl<'l> ClosureSlots<'l> {
    pub fn new(lambda: &'l TypedLambda, closure: &'l TypedExpr, closure_use: ClosureUse) -> Self {
        Self {
            lambda,
            closure,
            closure_use,
        }
    }

    pub fn free_parameters(&self) -> impl Iterator<Item = BindingId> + 'l {
        let free = self.closure_use == ClosureUse::Free;
        self.lambda
            .params
            .iter()
            .filter(move |_| free)
            .map(|parameter| parameter.binding)
    }

    pub fn list(&self, listing: &mut SlotListing) {
        for parameter in &self.lambda.params {
            listing.list(&self.parameter(parameter.binding), &parameter.ty);
        }
        listing.list(&self.result(), self.result_type());
    }

    pub fn close(&self, context: &mut WalkContext<'_, '_>, body: &TextValue) -> TextValue {
        let result = self.result();
        context.receive(&Slot::from(result.clone()), body);
        let parameters = self
            .lambda
            .params
            .iter()
            .map(|parameter| (self.parameter(parameter.binding), &parameter.ty))
            .collect();
        let slots = FunctionSlots::new(parameters, (result, self.result_type()));
        let mut value = TextValue::default();
        for (path, slot, position) in slots.positions() {
            let node = match self.binder_of(&slot) {
                Some(binder) => self.parameter_node(context, (&path, binder), (slot, &position)),
                None => context.node(slot),
            };
            value = value.nesting(path, node);
        }
        value
    }

    fn binder_of(&self, slot: &Slot) -> Option<&'l TypedBinder> {
        self.lambda
            .params
            .iter()
            .find(|binder| self.parameter(binder.binding) == *slot.owner())
    }

    fn parameter_node(
        &self,
        context: &mut WalkContext<'_, '_>,
        (path, binder): (&TypePath, &TypedBinder),
        (parameter, position): (Slot, &TextPosition),
    ) -> NodeId {
        if self.closure_use == ClosureUse::Free {
            if position.path.is_top() && binder.written_view {
                context.require(parameter.clone(), Need::View);
            }
            return context.node(parameter);
        }
        let given = Slot::new(SlotOwner::Value(self.closure.span), path.clone());
        let transfer = if position.is_written_reference() {
            Transfer::Refer
        } else {
            Transfer::Move
        };
        let node = context.node(given);
        let handed = TextValue::from(Source::Slot(node), self.closure.span).transferred(transfer);
        context.receive(&parameter, &handed);
        node
    }

    fn parameter(&self, binding: BindingId) -> SlotOwner {
        match self.closure_use {
            ClosureUse::Free => SlotOwner::Parameter(binding),
            ClosureUse::Fixed => SlotOwner::Local(binding),
        }
    }

    pub fn result(&self) -> SlotOwner {
        SlotOwner::ClosureResult(self.closure.span)
    }

    pub fn result_type(&self) -> &'l Type {
        match &self.closure.ty {
            Type::Function(_, _, result) => result,
            other => other,
        }
    }
}

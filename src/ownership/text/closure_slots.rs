use super::node_id::NodeId;
use super::slot::Slot;
use super::slot_listing::SlotListing;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_position::{TextPosition, TextPositions};
use super::text_value::TextValue;
use super::transfer::Transfer;
use super::type_path::TypePath;
use super::walk_context::WalkContext;
use crate::core::BindingId;
use crate::types::{Type, TypedExpr, TypedLambda};

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
            .enumerate()
            .flat_map(|(part, parameter)| {
                let base = TypePath::default().child(part);
                TextPositions::of(&parameter.ty)
                    .into_iter()
                    .map(move |position| (base.joined(&position.path), parameter.binding, position))
            });
        let mut value = TextValue::default();
        for (path, binding, position) in parameters.collect::<Vec<_>>() {
            let node = self.parameter_node(context, (&path, binding), &position);
            value = value.nesting(path, node);
        }
        for position in TextPositions::of(self.result_type()) {
            let node = context.node(Slot::new(result.clone(), position.path.clone()));
            value = value.nesting(TypePath::default().result().joined(&position.path), node);
        }
        value
    }

    fn parameter_node(
        &self,
        context: &mut WalkContext<'_, '_>,
        (path, binding): (&TypePath, BindingId),
        position: &TextPosition,
    ) -> NodeId {
        let parameter = Slot::new(self.parameter(binding), position.path.clone());
        if self.closure_use == ClosureUse::Free {
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

    fn result_type(&self) -> &'l Type {
        match &self.closure.ty {
            Type::Function(_, _, result) => result,
            other => other,
        }
    }
}

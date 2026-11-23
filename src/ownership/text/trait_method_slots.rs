use super::function_slots::FunctionSlots;
use super::need::Need;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::walk_context::WalkContext;
use crate::core::BindingId;
use crate::types::{Type, TypedBinder, TypedLambda};

pub struct TraitMethodSlots<'c, 'w, 'p> {
    context: &'c mut WalkContext<'w, 'p>,
    declared: BindingId,
}

impl<'c, 'w, 'p> TraitMethodSlots<'c, 'w, 'p> {
    pub fn new(context: &'c mut WalkContext<'w, 'p>, declared: BindingId) -> Self {
        Self { context, declared }
    }

    pub fn share(self, body: BindingId, (lambda, result): (&TypedLambda, &Type)) {
        let receiver = self.context.program().written().has_receiver(body);
        let parameters = lambda
            .params
            .iter()
            .map(|parameter| (SlotOwner::Parameter(parameter.binding), &parameter.ty))
            .collect();
        let written = lambda.params.iter().skip(usize::from(receiver));
        for parameter in
            written.filter(|parameter| TypedBinder::writes_a_view(None, Some(&parameter.ty)))
        {
            let pinned = Slot::from(SlotOwner::Parameter(parameter.binding));
            self.context.require(pinned, Need::View);
        }
        let slots = FunctionSlots::new(parameters, (SlotOwner::Result(body), result));
        for (path, own, _) in slots.positions() {
            if receiver && path.below(0).is_some() {
                continue;
            }
            let own = self.context.node(own);
            let shared = self
                .context
                .node(Slot::new(SlotOwner::TraitMethod(self.declared), path));
            self.context.unite(own, shared);
        }
    }
}

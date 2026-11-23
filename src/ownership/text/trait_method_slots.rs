use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::text_position::TextPositions;
use super::type_path::TypePath;
use super::walk_context::WalkContext;
use crate::core::BindingId;
use crate::types::{Type, TypedLambda};

pub struct TraitMethodSlots<'c, 'w, 'p> {
    context: &'c mut WalkContext<'w, 'p>,
    declared: BindingId,
}

impl<'c, 'w, 'p> TraitMethodSlots<'c, 'w, 'p> {
    pub fn new(context: &'c mut WalkContext<'w, 'p>, declared: BindingId) -> Self {
        Self { context, declared }
    }

    pub fn share(mut self, body: BindingId, (lambda, result): (&TypedLambda, &Type)) {
        let skipped = usize::from(self.context.program().written().has_receiver(body));
        for (part, parameter) in lambda.params.iter().enumerate().skip(skipped) {
            let owner = SlotOwner::Parameter(parameter.binding);
            self.unite(&owner, &parameter.ty, &TypePath::default().child(part));
        }
        self.unite(
            &SlotOwner::Result(body),
            result,
            &TypePath::default().result(),
        );
    }

    fn unite(&mut self, owner: &SlotOwner, ty: &Type, base: &TypePath) {
        for position in TextPositions::of(ty) {
            let own = self
                .context
                .node(Slot::new(owner.clone(), position.path.clone()));
            let shared = Slot::new(
                SlotOwner::TraitMethod(self.declared),
                base.joined(&position.path),
            );
            let shared = self.context.node(shared);
            self.context.unite(own, shared);
        }
    }
}

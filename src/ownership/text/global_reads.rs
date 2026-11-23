use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::text_position::TextPositions;
use super::text_read::TextRead;
use super::text_value::TextValue;
use super::transfer::Transfer;
use super::type_path::TypePath;
use super::walk_context::WalkContext;
use crate::core::BindingId;
use crate::types::{Type, TypedExpr, TypedExprKind, TypedLambda};

pub struct GlobalReads<'c, 'w, 'p> {
    context: &'c mut WalkContext<'w, 'p>,
    expr: &'c TypedExpr,
}

impl<'c, 'w, 'p> GlobalReads<'c, 'w, 'p> {
    pub fn new(context: &'c mut WalkContext<'w, 'p>, expr: &'c TypedExpr) -> Self {
        Self { context, expr }
    }

    pub fn value(self, binding: BindingId) -> TextValue {
        let program = self.context.program();
        if program.written().is_value(binding) {
            let global = Slot::from(SlotOwner::Global(binding));
            return self
                .context
                .read(&global, TextRead::of(self.expr, Transfer::Refer));
        }
        match program
            .definition(binding)
            .map(|definition| &definition.value.kind)
        {
            Some(TypedExprKind::Fn(lambda)) => self.function(binding, lambda),
            _ => TextValue::default(),
        }
    }

    fn function(self, binding: BindingId, lambda: &TypedLambda) -> TextValue {
        let Type::Function(_, parameters, result) = &self.expr.ty else {
            return TextValue::default();
        };
        let owners = lambda
            .params
            .iter()
            .zip(parameters)
            .enumerate()
            .map(|(part, (parameter, ty))| {
                let owner = SlotOwner::Parameter(parameter.binding);
                (TypePath::default().child(part), owner, ty)
            })
            .chain([(
                TypePath::default().result(),
                SlotOwner::Result(binding),
                result.as_ref(),
            )]);
        let mut value = TextValue::default();
        for (base, owner, ty) in owners.collect::<Vec<_>>() {
            for position in TextPositions::of(ty) {
                let node = self
                    .context
                    .node(Slot::new(owner.clone(), position.path.clone()));
                value = value.nesting(base.joined(&position.path), node);
            }
        }
        value
    }
}

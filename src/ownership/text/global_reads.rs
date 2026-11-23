use super::function_slots::FunctionSlots;
use super::reaching_text::ReachingText;
use super::representation::Representation;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_position::TextPositions;
use super::text_read::TextRead;
use super::text_value::TextValue;
use super::transfer::Transfer;
use super::walk_context::WalkContext;
use crate::core::BindingId;
use crate::types::{Type, TypedExpr, TypedExprKind, TypedLambda, TypedLeaf};

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
            Some(TypedExprKind::Leaf(TypedLeaf::Rust(..))) => self.template(),
            _ => TextValue::default(),
        }
    }

    fn function(self, binding: BindingId, lambda: &TypedLambda) -> TextValue {
        let Type::Function(_, parameters, result) = &self.expr.ty else {
            return TextValue::default();
        };
        let parameters = lambda
            .params
            .iter()
            .zip(parameters)
            .map(|(parameter, ty)| (SlotOwner::Parameter(parameter.binding), ty))
            .collect();
        let result = (SlotOwner::Result(binding), result.as_ref());
        FunctionSlots::new(parameters, result).value(self.context)
    }

    fn template(self) -> TextValue {
        let Type::Function(_, _, result) = &self.expr.ty else {
            return TextValue::default();
        };
        let created = TextPositions::of(result)
            .into_iter()
            .filter(|position| position.written == Representation::Owned);
        let mut value = TextValue::default();
        for position in created.collect::<Vec<_>>() {
            let path = FunctionSlots::result_path().joined(&position.path);
            let slot = Slot::new(SlotOwner::Value(self.expr.span), path.clone());
            let made = TextValue::from(Source::Text(ReachingText::OWNED), self.expr.span);
            self.context.receive(&slot, &made);
            value = value.nesting(path, self.context.node(slot));
        }
        value
    }
}

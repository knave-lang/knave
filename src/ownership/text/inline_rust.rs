use super::call_target::CallTarget;
use super::callee_scheme::CalleeScheme;
use super::inline_flows::InlineFlows;
use super::representation::Representation;
use super::slot_owner::SlotOwner;
use super::template_call::TemplateCall;
use super::text_position::TextPositions;
use super::text_value::TextValue;
use super::walk_context::WalkContext;
use crate::source::Span;
use crate::types::TypedExpr;

pub struct InlineRust<'t> {
    template: TemplateCall<'t>,
    queries: Vec<usize>,
}

impl<'t> InlineRust<'t> {
    pub fn new(scheme: CalleeScheme<'t>, queries: Vec<usize>) -> Self {
        Self {
            template: TemplateCall::new(scheme),
            queries,
        }
    }

    fn shares_nested_text(call: &TypedExpr, values: &[TextValue]) -> bool {
        values.iter().any(|value| !value.nested().is_empty())
            || TextPositions::of(&call.ty)
                .iter()
                .any(|position| !position.path.is_top())
    }
}

impl<'t> CallTarget<'t> for InlineRust<'t> {
    fn scheme(&self) -> &CalleeScheme<'t> {
        self.template.scheme()
    }

    fn inline_value(
        &self,
        context: &mut WalkContext<'_, '_>,
        call: &TypedExpr,
        values: &[TextValue],
    ) -> Option<TextValue> {
        Self::shares_nested_text(call, values)
            .then(|| InlineFlows::new(context, call).value(values, &self.queries))
    }

    fn converted(&self, values: &[TextValue]) -> Option<TextValue> {
        self.template.converted(values)
    }

    fn owned_argument(&self, call: Span) -> Option<SlotOwner> {
        self.template.owned_argument(call)
    }

    fn changes_in_place(&self) -> bool {
        self.template.changes_in_place()
    }

    fn made(
        &self,
        written: Representation,
        arguments: &[TextValue],
        call: Span,
    ) -> Option<TextValue> {
        self.template.made(written, arguments, call)
    }
}

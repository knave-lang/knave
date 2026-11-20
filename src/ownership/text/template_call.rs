use super::call_target::CallTarget;
use super::callee_scheme::CalleeScheme;
use super::reaching_text::ReachingText;
use super::representation::Representation;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_conversion::TextConversion;
use super::text_value::TextValue;
use super::transfer::Transfer;
use crate::source::Span;

pub struct TemplateCall<'t> {
    scheme: CalleeScheme<'t>,
}

impl<'t> TemplateCall<'t> {
    pub fn new(scheme: CalleeScheme<'t>) -> Self {
        Self { scheme }
    }
}

impl<'t> CallTarget<'t> for TemplateCall<'t> {
    fn scheme(&self) -> &CalleeScheme<'t> {
        &self.scheme
    }

    fn converted(&self, values: &[TextValue]) -> Option<TextValue> {
        let (parameters, result) = self.scheme.signature()?;
        TextConversion::is_conversion(parameters, result)
            .then(|| TextConversion::converted(&values[0]))
    }

    fn owned_argument(&self, call: Span) -> Option<SlotOwner> {
        Some(SlotOwner::Argument(call))
    }

    fn changes_in_place(&self) -> bool {
        true
    }

    fn made(
        &self,
        written: Representation,
        arguments: &[TextValue],
        call: Span,
    ) -> Option<TextValue> {
        Some(match written {
            Representation::Owned => TextValue::from(Source::Text(ReachingText::OWNED), call),
            _ => arguments
                .iter()
                .fold(TextValue::default(), |value, argument| {
                    value.merged(argument.clone().transferred(Transfer::View))
                }),
        })
    }
}

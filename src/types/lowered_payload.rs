use super::enums::VariantPayload;
use crate::diagnostic::Diagnostic;

pub(super) struct LoweredPayload {
    pub(super) payload: VariantPayload,
    pub(super) problems: Vec<Diagnostic>,
    pub(super) bare_text: Vec<usize>,
}

use super::enums::VariantPayload;
use super::payload_lowering::PayloadLowering;
use crate::diagnostic::Diagnostic;

pub(super) struct LoweredPayload {
    pub(super) payload: VariantPayload,
    pub(super) problems: Vec<Diagnostic>,
}

impl LoweredPayload {
    pub(super) fn lowering(&self) -> PayloadLowering {
        match self.problems.is_empty() {
            true => PayloadLowering::Complete,
            false => PayloadLowering::Failed,
        }
    }
}

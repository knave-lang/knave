use super::RustType;
use super::enums::EnumDefinition;
use crate::diagnostic::Diagnostic;

pub(super) struct CollectedEnum {
    pub(super) definition: EnumDefinition,
    pub(super) rust_type: Option<RustType>,
    pub(super) problems: Vec<Diagnostic>,
}

impl CollectedEnum {
    pub(super) fn new(
        definition: EnumDefinition,
        rust_type: Option<RustType>,
        problems: Vec<Diagnostic>,
    ) -> Self {
        Self {
            definition,
            rust_type,
            problems,
        }
    }
}

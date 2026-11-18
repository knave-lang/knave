use super::StructDefinition;
use crate::diagnostic::Diagnostic;

pub(super) struct CollectedStruct {
    pub(super) definition: StructDefinition,
    pub(super) problem: Option<Diagnostic>,
}

impl CollectedStruct {
    pub(super) fn new(definition: StructDefinition, problem: Option<Diagnostic>) -> Self {
        Self {
            definition,
            problem,
        }
    }
}

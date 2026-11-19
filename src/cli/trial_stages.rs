use crate::check::ProgramChecker;
use crate::diagnostic::Diagnostics;
use crate::emit::RustEmitter;
use crate::expand::Program;
use crate::ownership::EnforcedOwnership;
use crate::source::Span;
use crate::types::TypedProgram;

pub struct TrialStages;

impl TrialStages {
    pub fn check(program: &Program) -> Result<TypedProgram, Diagnostics> {
        ProgramChecker::check::<EnforcedOwnership>(program)
    }

    pub fn emit(typed: &TypedProgram, entry: Span) -> Option<String> {
        RustEmitter::emit(typed, entry)
            .ok()
            .map(|emitted| emitted.code)
    }
}

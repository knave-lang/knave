use crate::diagnostic::Diagnostics;
use crate::types::{InferredCopies, TypedProgram};

use super::OwnershipChecker;

pub trait OwnershipGate {
    fn inferred_copies(program: &mut TypedProgram) -> Result<InferredCopies, Diagnostics>;
}

pub struct EnforcedOwnership;

pub struct WaivedOwnership;

impl OwnershipGate for EnforcedOwnership {
    fn inferred_copies(program: &mut TypedProgram) -> Result<InferredCopies, Diagnostics> {
        OwnershipChecker::check(program)
    }
}

impl OwnershipGate for WaivedOwnership {
    fn inferred_copies(program: &mut TypedProgram) -> Result<InferredCopies, Diagnostics> {
        Ok(OwnershipChecker::check_without_refusals(program))
    }
}

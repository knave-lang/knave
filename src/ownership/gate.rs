use crate::diagnostic::Diagnostics;
use crate::types::{
    DroppedInlineSchemes, InferredCopies, InlineSchemes, KeptInlineSchemes, TypedProgram,
};

use super::OwnershipChecker;

pub trait OwnershipGate {
    type Schemes: InlineSchemes;

    fn inferred_copies(program: &mut TypedProgram) -> Result<InferredCopies, Diagnostics>;
}

pub struct EnforcedOwnership;

pub struct WaivedOwnership;

pub struct TextReportOwnership;

impl OwnershipGate for EnforcedOwnership {
    type Schemes = DroppedInlineSchemes;

    fn inferred_copies(program: &mut TypedProgram) -> Result<InferredCopies, Diagnostics> {
        OwnershipChecker::check(program)
    }
}

impl OwnershipGate for WaivedOwnership {
    type Schemes = DroppedInlineSchemes;

    fn inferred_copies(program: &mut TypedProgram) -> Result<InferredCopies, Diagnostics> {
        Ok(OwnershipChecker::waived(program))
    }
}

impl OwnershipGate for TextReportOwnership {
    type Schemes = KeptInlineSchemes;

    fn inferred_copies(program: &mut TypedProgram) -> Result<InferredCopies, Diagnostics> {
        WaivedOwnership::inferred_copies(program)
    }
}

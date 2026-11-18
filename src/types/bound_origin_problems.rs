use super::bound_origins::{BoundOrigin, BoundOrigins};
use crate::diagnostic::{Diagnostic, Label};
use crate::source::Span;

pub(super) struct BoundOriginProblems;

impl BoundOriginProblems {
    pub(super) fn annotate(diagnostic: Diagnostic, origins: &BoundOrigins) -> Diagnostic {
        let origins = origins.as_slice();
        let labeled = origins.iter().fold(diagnostic, Self::with_origin_label);
        match origins.first() {
            Some(BoundOrigin::BorrowCopied(_)) => labeled.with_help(Self::clone_copied_help()),
            Some(BoundOrigin::ImplBody(_)) | None => labeled,
        }
    }

    fn with_origin_label(diagnostic: Diagnostic, origin: &BoundOrigin) -> Diagnostic {
        match origin {
            BoundOrigin::BorrowCopied(copied) => {
                diagnostic.with_label(Self::borrow_copied(*copied))
            }
            BoundOrigin::ImplBody(needed) => {
                diagnostic.with_label(Self::needed_by_impl_body(*needed))
            }
        }
    }

    fn borrow_copied(span: Span) -> Label {
        Label::new(span, "this copies the borrowed value")
    }

    fn needed_by_impl_body(span: Span) -> Label {
        Label::new(span, "the body of the impl needs it here")
    }

    fn clone_copied_help() -> String {
        "to copy a value that is not `copy`, write `(clone ...)` around it".to_string()
    }
}

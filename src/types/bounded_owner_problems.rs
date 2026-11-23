use std::rc::Rc;

use crate::core::Keyword;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub(super) struct BoundedOwnerProblems<'b> {
    owner: &'b str,
    bound_fields: String,
}

impl<'b> BoundedOwnerProblems<'b> {
    pub(super) fn new(owner: &'b str, fields: &[Rc<str>]) -> Self {
        let marked: Vec<String> = (fields.iter())
            .map(|field| format!("`{}{field}`", Keyword::MARKER))
            .collect();
        Self {
            owner,
            bound_fields: marked.join(", "),
        }
    }

    pub(super) fn unbound_read(&self, field: &str, span: Span) -> Diagnostic {
        let owner = self.owner;
        Diagnostic::error(
            DiagnosticCode::NO_FIELDS,
            format!(
                "no `has` bound on `{owner}` names `{}{field}`",
                Keyword::MARKER
            ),
            Label::new(
                span,
                format!("the bounds on `{owner}` read {}", self.bound_fields),
            ),
        )
        .with_help(format!(
            "add `(has {}{field} type)` to the bounds on `{owner}`",
            Keyword::MARKER
        ))
    }

    pub(super) fn update(&self, span: Span) -> Diagnostic {
        let owner = self.owner;
        Diagnostic::error(
            DiagnosticCode::NO_FIELDS,
            format!("cannot update `{owner}` through its `has` bounds"),
            Label::new(
                span,
                format!("the bounds on `{owner}` only read {}", self.bound_fields),
            ),
        )
        .with_help("name the struct in the signature to build an updated copy")
    }
}

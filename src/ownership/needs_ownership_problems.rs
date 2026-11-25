use std::rc::Rc;

use super::problems::OwnershipProblems;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct MacroFieldRead<'n> {
    pub macro_name: Rc<str>,
    pub field: Rc<str>,
    pub field_span: Span,
    pub field_type: String,
    pub callee: Option<&'n str>,
}

pub struct NeedsOwnershipProblems;

impl NeedsOwnershipProblems {
    const BORROWED_ARGUMENT_TITLE: &'static str =
        "this value is borrowed, but the call needs to own it";

    pub fn needs_ownership(span: Span) -> Diagnostic {
        Self::borrowed_argument(span).with_help("pass a copy with `(clone ...)`")
    }

    pub fn macro_field_needs_ownership(read: &MacroFieldRead<'_>) -> Diagnostic {
        let MacroFieldRead {
            macro_name, field, ..
        } = read;
        let borrowed = format!("(ref {})", read.field_type);
        Diagnostic::error(
            DiagnosticCode::NEEDS_OWNERSHIP,
            Self::BORROWED_ARGUMENT_TITLE,
            Label::new(
                read.field_span,
                format!("`{macro_name}` passes this field as a borrow"),
            ),
        )
        .with_help(format!(
            "`{macro_name}` passes `:{field}` as a `{borrowed}`; {}",
            Self::borrowed_field_fix(read, &borrowed)
        ))
    }

    fn borrowed_field_fix(read: &MacroFieldRead<'_>, borrowed: &str) -> String {
        let field = &read.field;
        match read.callee {
            Some(callee) => format!(
                "take a borrow and clone it, as in `(fn (({field} {borrowed})) ({callee} (clone {field})))`"
            ),
            None => format!(
                "annotate the parameter as `{borrowed}` and clone it before the owning call"
            ),
        }
    }

    pub fn needs_pipeline_ownership(span: Span) -> Diagnostic {
        OwnershipProblems::pipeline_wording(Self::borrowed_argument(span), "this value", "...")
    }

    fn borrowed_argument(span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::NEEDS_OWNERSHIP,
            Self::BORROWED_ARGUMENT_TITLE,
            Label::new(span, "only a borrow is available here"),
        )
    }
}

use std::rc::Rc;

use super::assigned_place::AssignedPlace;
use super::field_read_use::FieldReadUse;
use super::problems::OwnershipProblems;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub struct MacroFieldRead<'n> {
    pub macro_name: Rc<str>,
    pub field: Rc<str>,
    pub field_span: Span,
    pub field_type: String,
    pub used: FieldReadUse<'n>,
}

pub struct NeedsOwnershipProblems;

impl NeedsOwnershipProblems {
    const DISTINCT_PARAMETER_SUFFIX: &'static str = "-value";
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
        match &read.used {
            FieldReadUse::Call(callee) => format!(
                "take a borrow and clone it, as in `(fn (({field} {borrowed})) ({callee} (clone {field})))`"
            ),
            FieldReadUse::Assignment(place) => {
                let parameter = Self::parameter_beside(field, place);
                let written = &place.written;
                format!(
                    "take a borrow and clone it, as in `(fn (({parameter} {borrowed})) (set! {written} (clone {parameter})))`"
                )
            }
            FieldReadUse::UnwrittenAssignment => {
                format!("annotate the parameter as `{borrowed}` and clone it before you assign it")
            }
            FieldReadUse::Unnamed => format!(
                "annotate the parameter as `{borrowed}` and clone it before the owning call"
            ),
        }
    }

    fn parameter_beside(field: &str, place: &AssignedPlace<'_>) -> String {
        if place.locals.contains(&field) {
            format!("{field}{}", Self::DISTINCT_PARAMETER_SUFFIX)
        } else {
            field.to_string()
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

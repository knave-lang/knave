use std::rc::Rc;

use super::problems::TypeProblems;
use super::unifier::LiteralKind;
use crate::core::{FieldLabel, Keyword};
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub(super) struct FieldOwnerProblems;

impl FieldOwnerProblems {
    pub(super) fn ambiguous(field: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::CANNOT_INFER,
            "type annotations needed",
            Label::new(
                span,
                format!("cannot tell which struct's `{field}` this is"),
            ),
        )
        .with_help("name the struct in a signature, as in `(fn ((ref point)) _)`")
    }

    pub(super) fn undeclared(label: &FieldLabel, similar: Option<Rc<str>>) -> Diagnostic {
        let absent = Diagnostic::error(
            DiagnosticCode::UNKNOWN_FIELD,
            format!("no struct has a field `{}`", label.spelled(&label.name)),
            Label::new(label.span, "unknown field"),
        );
        TypeProblems::with_similar_field(absent, label, similar)
    }

    pub(super) fn read_mismatch(
        (owner, field): (&str, &str),
        (read, passed): (String, String),
        span: Span,
    ) -> Diagnostic {
        let field = format!("{}{field}", Keyword::MARKER);
        Diagnostic::error(
            DiagnosticCode::MISSING_TRAIT,
            format!("`{owner}` reads `{field}` as `{read}`, but this call passes `{passed}`"),
            Label::new(span, format!("this `{owner}` reads `{field}` as `{read}`")),
        )
        .with_help(format!(
            "pass a `{read}`, or write `{read}` as the read of the bound"
        ))
    }

    pub(super) fn undeclared_written(field: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::UNKNOWN_FIELD,
            format!("no struct has a field `{}{field}`", Keyword::MARKER),
            Label::new(span, "this bound names it"),
        )
    }

    pub(super) fn unshared_written(field: &str, owner: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::UNSHARED_FIELD_BOUND,
            format!("only `{owner}` has a field `{}{field}`", Keyword::MARKER),
            Label::new(span, "this bound names it"),
        )
        .with_help(format!(
            "write `{owner}` in the signature in place of the bound variable"
        ))
    }

    pub(super) fn literal_owner(literal: LiteralKind, span: Span) -> Diagnostic {
        TypeProblems::no_fields(literal.default_type().names().shown(), span)
    }
}

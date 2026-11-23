use std::rc::Rc;

use super::problems::TypeProblems;
use super::refusal_wording::RefusalWording;
use super::shown_read_mismatch::ShownReadMismatch;
use super::unifier::LiteralKind;
use super::written_refusal::WrittenRefusal;
use crate::core::{FieldLabel, Keyword};
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub(super) struct FieldOwnerProblems;

impl FieldOwnerProblems {
    const NAME_THE_STRUCT: &'static str =
        "name the struct in a signature, as in `(fn ((ref point)) _)`";

    pub(super) fn ambiguous(field: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::CANNOT_INFER,
            "type annotations needed",
            Label::new(
                span,
                format!("cannot tell which struct's `{field}` this is"),
            ),
        )
        .with_help(Self::NAME_THE_STRUCT)
    }

    pub(super) fn undeclared(label: &FieldLabel, similar: Option<Rc<str>>) -> Diagnostic {
        let absent = Diagnostic::error(
            DiagnosticCode::UNKNOWN_FIELD,
            format!("no struct has a field `{}`", label.spelled(&label.name)),
            Label::new(label.span, "unknown field"),
        );
        TypeProblems::with_similar_field(absent, label, similar)
    }

    pub(super) fn read_mismatch(mismatch: &ShownReadMismatch, span: Span) -> Diagnostic {
        let ShownReadMismatch {
            owner,
            field,
            read,
            bound,
        } = mismatch;
        let field = format!("{}{field}", Keyword::MARKER);
        Diagnostic::error(
            DiagnosticCode::MISSING_TRAIT,
            format!(
                "`{owner}` reads `{field}` as `{read}`, but the `has` bound reads it as `{bound}`"
            ),
            Label::new(span, format!("this `{owner}` reads `{field}` as `{read}`")),
        )
        .with_help(format!("pass a struct whose `{field}` reads as `{bound}`"))
    }

    pub(super) fn unkept(refusal: WrittenRefusal, field: &str, span: Span) -> Diagnostic {
        let (title, help) = RefusalWording::written().of(refusal, field);
        Diagnostic::error(
            DiagnosticCode::UNKEPT_FIELD_BOUND,
            title,
            Label::new(span, "this read goes through the `has` bound"),
        )
        .with_help(help)
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

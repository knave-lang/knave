use std::rc::Rc;

use super::problems::TypeProblems;
use super::shown_read_mismatch::ShownReadMismatch;
use super::unifier::LiteralKind;
use super::written_refusal::WrittenRefusal;
use crate::core::{FieldLabel, Keyword};
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub(super) struct FieldOwnerProblems;

impl FieldOwnerProblems {
    const FIELD: &'static str = "{field}";

    const UNKEPT_WORDING: [(&'static str, &'static str); 8] = [
        (
            "a `has` bound cannot read the function field `{field}`",
            "name the struct in the signature to call its function field",
        ),
        (
            "the read of `{field}` in this `has` bound names a type variable",
            "write the field's type in the bound, as in `(has {field} int)`",
        ),
        (
            "a struct with an open type argument reaches this `has` bound on `{field}`",
            "pass the struct with its type arguments fixed, or name the struct",
        ),
        (
            "another `has` bound reads `{field}` as a different type",
            "write `{field}` with one read type in every `has` bound",
        ),
        (
            "this function changes `{field}` through a `has` bound",
            "name the struct in the signature to change its fields",
        ),
        (
            "this function moves a value that it reads `{field}` from through a `has` bound",
            "take the value as `(ref a)`, or name the struct in the signature",
        ),
        (
            "a borrow read from `{field}` through a `has` bound leaves this function",
            "return or store an owned value, or name the struct in the signature",
        ),
        (
            "the trait or getter for `{field}` would take a name the program already uses",
            "rename the type, trait or method that takes the name, or name the struct",
        ),
    ];

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

    pub(super) fn read_mismatch(mismatch: &ShownReadMismatch, span: Span) -> Diagnostic {
        let ShownReadMismatch {
            owner,
            field,
            read,
            passed,
        } = mismatch;
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

    pub(super) fn unkept(refusal: WrittenRefusal, field: &str, span: Span) -> Diagnostic {
        let field = format!("{}{field}", Keyword::MARKER);
        let (title, help) = Self::UNKEPT_WORDING[refusal as usize];
        Diagnostic::error(
            DiagnosticCode::UNKEPT_FIELD_BOUND,
            title.replace(Self::FIELD, &field),
            Label::new(span, "this read goes through the `has` bound"),
        )
        .with_help(help.replace(Self::FIELD, &field))
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

use std::rc::Rc;

use super::problems::TypeProblems;
use super::shown_read_mismatch::ShownReadMismatch;
use super::unifier::LiteralKind;
use super::written_refusal::{ReadRefusal, UseRefusal, WrittenRefusal};
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
        let (title, help) = Self::unkept_wording(refusal, &field);
        Diagnostic::error(
            DiagnosticCode::UNKEPT_FIELD_BOUND,
            title,
            Label::new(span, "this read goes through the `has` bound"),
        )
        .with_help(help)
    }

    fn unkept_wording(refusal: WrittenRefusal, field: &str) -> (String, String) {
        match refusal {
            WrittenRefusal::Read(read) => Self::read_wording(read, field),
            WrittenRefusal::Use(using) => Self::use_wording(using, field),
        }
    }

    fn read_wording(refusal: ReadRefusal, field: &str) -> (String, String) {
        match refusal {
            ReadRefusal::Function => (
                format!("a `has` bound cannot read the function field `{field}`"),
                "name the struct in the signature to call its function field".to_string(),
            ),
            ReadRefusal::Open => (
                format!("the read of `{field}` in this `has` bound names a type variable"),
                format!("write the field's type in the bound, as in `(has {field} int)`"),
            ),
            ReadRefusal::OpenStruct => (
                format!(
                    "a struct with an open type argument reaches this `has` bound on `{field}`"
                ),
                "pass the struct with its type arguments fixed, or name the struct".to_string(),
            ),
            ReadRefusal::Otherwise => (
                format!("another `has` bound reads `{field}` as a different type"),
                format!("write `{field}` with one read type in every `has` bound"),
            ),
        }
    }

    fn use_wording(refusal: UseRefusal, field: &str) -> (String, String) {
        match refusal {
            UseRefusal::ChangesField => (
                format!("this function changes `{field}` through a `has` bound"),
                "name the struct in the signature to change its fields".to_string(),
            ),
            UseRefusal::MovesOwner => (
                format!(
                    "this function moves a value that it reads `{field}` from through a `has` bound"
                ),
                "take the value as `(ref a)`, or name the struct in the signature".to_string(),
            ),
            UseRefusal::BorrowLeaves => (
                format!("a borrow read from `{field}` through a `has` bound leaves this function"),
                "return or store an owned value, or name the struct in the signature".to_string(),
            ),
            UseRefusal::NameTaken => (
                format!(
                    "the trait or getter for `{field}` would take a name the program already uses"
                ),
                "rename the type, trait or method that takes the name, or name the struct"
                    .to_string(),
            ),
        }
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

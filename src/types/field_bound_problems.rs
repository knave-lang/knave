use super::missing_read::MissingRead;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub(super) struct FieldBoundProblems {
    field_trait: String,
    generic: String,
    span: Span,
}

impl FieldBoundProblems {
    pub(super) fn new(field_trait: &str, generic: &str, span: Span) -> Self {
        Self {
            field_trait: field_trait.to_string(),
            generic: generic.to_string(),
            span,
        }
    }

    pub(super) fn missing(&self, read: &MissingRead) -> Diagnostic {
        match read {
            MissingRead::Open(read) => self.missing_open(read),
            MissingRead::Known(read) => self.missing_known(read),
        }
    }

    fn missing_open(&self, read: &str) -> Diagnostic {
        let bound = self.bound(read);
        self.unproven(&bound).with_help(format!(
            "add the trait and its read to the signature's `where`: `({} {bound}) {read}`",
            self.generic
        ))
    }

    fn missing_known(&self, read: &str) -> Diagnostic {
        let bound = self.bound(read);
        self.unproven(&bound).with_help(format!(
            "add the trait to the signature's `where`: `({} {bound})`",
            self.generic
        ))
    }

    fn unproven(&self, bound: &str) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::MISSING_BOUND,
            format!("`{}` might not implement `{bound}`", self.generic),
            Label::new(self.span, format!("this needs `{bound}`")),
        )
    }

    fn bound(&self, read: &str) -> String {
        format!("({} {read})", self.field_trait)
    }
}

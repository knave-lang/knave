use super::display::TypeDisplay;
use super::field_reader::FieldReader;
use super::refusal_wording::RefusalWording;
use super::shown_read_mismatch::ShownReadMismatch;
use super::typed::TypedProgram;
use super::written_refusal::WrittenRefusal;
use crate::core::Keyword;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub(super) struct InferredBoundProblems<'f> {
    field: &'f str,
    read: Span,
}

impl<'f> InferredBoundProblems<'f> {
    const TITLE: &'static str = "type annotations needed";
    const MISMATCH_TITLE: &'static str = "mismatched types";

    pub(super) fn new(field: &'f str, read: Span) -> Self {
        Self { field, read }
    }

    pub(super) fn unkept(&self, refusal: WrittenRefusal, bound: Option<&str>) -> Diagnostic {
        let (cause, help) = RefusalWording::inferred().of(refusal, self.field);
        let inferred = match bound {
            Some(bound) => format!("the bound `{bound}`"),
            None => "a `has` bound".to_string(),
        };
        let note = format!("{inferred} inferred from this read cannot be kept: {cause}");
        self.cannot_infer(note).with_help(help)
    }

    pub(super) fn shown_bound(reader: &FieldReader, program: &TypedProgram) -> String {
        let display = TypeDisplay::new(&program.bindings, &program.generics);
        let read = display.show(&reader.read);
        format!("(has {}{} {read})", Keyword::MARKER, reader.field)
    }

    pub(super) fn differing_reads(&self, reads: &[(String, String)]) -> Diagnostic {
        (self.cannot_infer(RefusalWording::differing_reads(self.field, reads)))
            .with_help(RefusalWording::ONE_READ_PER_FUNCTION)
    }

    pub(super) fn read_mismatch(
        &self,
        mismatch: &ShownReadMismatch,
        owner_fixed: Span,
    ) -> Diagnostic {
        let ShownReadMismatch {
            owner, read, bound, ..
        } = mismatch;
        let field = self.marked();
        Diagnostic::error(
            DiagnosticCode::MISMATCHED_TYPES,
            Self::MISMATCH_TITLE,
            Label::new(
                owner_fixed,
                format!("this `{owner}` reads `{field}` as `{read}`"),
            ),
        )
        .with_label(Label::new(self.read, self.read_label()))
        .with_note(format!("the function uses this read as `{bound}`"))
    }

    pub(super) fn lacking_field(&self, owner: &str, call: Span) -> Diagnostic {
        let field = self.marked();
        Diagnostic::error(
            DiagnosticCode::UNKNOWN_FIELD,
            format!("`{owner}` has no field `{field}`"),
            Label::new(call, format!("this has no field `{field}`")),
        )
        .with_label(Label::new(self.read, self.read_label()))
    }

    pub(super) fn no_fields(&self, owner: &str, call: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::NO_FIELDS,
            format!("`{owner}` has no fields"),
            Label::new(call, "only structs have fields"),
        )
        .with_label(Label::new(self.read, self.read_label()))
    }

    fn cannot_infer(&self, note: String) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::CANNOT_INFER,
            Self::TITLE,
            Label::new(self.read, self.read_label()),
        )
        .with_note(note)
    }

    fn read_label(&self) -> String {
        format!("this read of `{}` infers a `has` bound", self.marked())
    }

    fn marked(&self) -> String {
        format!("{}{}", Keyword::MARKER, self.field)
    }
}

use std::fmt;

use super::arguments::Arguments;
use super::code_list::{CodeList, ListSize};
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::source::Span;

pub trait BuiltValue: Sized {
    const LIMIT: usize;

    fn built_size(&self) -> usize;

    fn too_large(builtin: &str, size: usize, span: Span) -> Diagnostic;

    fn refusal(label: String, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::MACRO_VALUE_SIZE,
            BuildLimit::TITLE,
            Label::new(span, label),
        )
    }
}

pub struct BuildLimit {
    builtin: &'static str,
    span: Span,
}

impl BuildLimit {
    pub const TITLE: &'static str = "macro code built a value too large";
    pub const TEXT_BYTES: usize = 1 << 24;
    pub const LIST_SIZE: usize = 1 << 22;
    pub const MODULE_NOTES: usize = 1 << 20;

    pub fn of(arguments: &Arguments) -> Self {
        Self {
            builtin: arguments.function(),
            span: arguments.span(),
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }

    #[inline]
    pub fn check<V: BuiltValue>(&self, value: V) -> Result<V, Diagnostic> {
        let size = value.built_size();
        if size <= V::LIMIT {
            Ok(value)
        } else {
            Err(V::too_large(self.builtin, size, self.span))
        }
    }
}

impl BuiltValue for String {
    const LIMIT: usize = BuildLimit::TEXT_BYTES;

    fn built_size(&self) -> usize {
        self.len()
    }

    fn too_large(builtin: &str, bytes: usize, span: Span) -> Diagnostic {
        Self::refusal(
            format!("`{builtin}` builds text of {bytes} bytes here"),
            span,
        )
        .with_note(format!(
            "text that macro code builds holds at most {} bytes",
            Self::LIMIT
        ))
        .with_help("build smaller text, or make the macro stop growing it")
    }
}

impl BuiltValue for CodeList {
    const LIMIT: usize = BuildLimit::LIST_SIZE;

    fn built_size(&self) -> usize {
        ListSize::of(self)
    }

    fn too_large(builtin: &str, size: usize, span: Span) -> Diagnostic {
        PlannedList::too_large(builtin, size, span)
    }
}

pub struct PlannedList(pub usize);

impl BuiltValue for PlannedList {
    const LIMIT: usize = BuildLimit::LIST_SIZE;

    fn built_size(&self) -> usize {
        self.0
    }

    fn too_large(builtin: &str, size: usize, span: Span) -> Diagnostic {
        Self::refusal(
            format!("`{builtin}` builds a list of {size} values here"),
            span,
        )
        .with_note(format!(
            "a list that macro code builds holds at most {} values, counting each value inside it",
            Self::LIMIT
        ))
        .with_help("build a smaller list, or make the macro stop growing it")
    }
}

pub struct HeldNotes(pub usize);

impl BuiltValue for HeldNotes {
    const LIMIT: usize = BuildLimit::MODULE_NOTES;

    fn built_size(&self) -> usize {
        self.0
    }

    fn too_large(builtin: &str, notes: usize, span: Span) -> Diagnostic {
        Self::refusal(format!("`{builtin}` adds module note {notes} here"), span)
            .with_note(format!(
                "a program holds at most {} module notes",
                Self::LIMIT
            ))
            .with_help("add fewer module notes")
    }
}

#[derive(Default)]
pub struct LimitedText {
    text: String,
    wanted: usize,
}

impl LimitedText {
    pub fn into_text(self) -> String {
        self.text
    }
}

impl fmt::Write for LimitedText {
    fn write_str(&mut self, part: &str) -> fmt::Result {
        self.wanted = self.wanted.saturating_add(part.len());
        if self.wanted <= BuildLimit::TEXT_BYTES {
            self.text.push_str(part);
            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}

impl BuiltValue for LimitedText {
    const LIMIT: usize = BuildLimit::TEXT_BYTES;

    fn built_size(&self) -> usize {
        self.wanted
    }

    fn too_large(builtin: &str, bytes: usize, span: Span) -> Diagnostic {
        String::too_large(builtin, bytes, span)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use super::{BuildLimit, HeldNotes, LimitedText, PlannedList};
    use crate::diagnostic::DiagnosticCode;
    use crate::interpret::arguments::{Arguments, BuiltinSite};
    use crate::interpret::{StepBudget, WorkMeter};
    use crate::source::{SourceId, Span};

    struct BuildLimitFixture;

    impl BuildLimitFixture {
        fn limit() -> BuildLimit {
            let span = Span::new(SourceId::new(0), 0..1);
            let site = BuiltinSite {
                function: "string",
                span,
            };
            let meter = WorkMeter::new(StepBudget::unmetered(), span);
            BuildLimit::of(&Arguments::new(site, Vec::new(), &meter))
        }

        fn text_of(bytes: usize) -> String {
            "x".repeat(bytes)
        }
    }

    #[test]
    fn text_at_the_limit_is_built() {
        let text = BuildLimitFixture::text_of(BuildLimit::TEXT_BYTES);
        assert!(BuildLimitFixture::limit().check(text).is_ok());
    }

    #[test]
    fn text_past_the_limit_is_refused() {
        let text = BuildLimitFixture::text_of(BuildLimit::TEXT_BYTES + 1);
        let refusal = BuildLimitFixture::limit()
            .check(text)
            .expect_err("too long");
        assert_eq!(refusal.code(), Some(&DiagnosticCode::MACRO_VALUE_SIZE));
    }

    #[test]
    fn limited_text_keeps_what_fits() {
        let mut text = LimitedText::default();
        text.write_str("ab").expect("it fits");
        let built = BuildLimitFixture::limit().check(text).expect("it fits");
        assert_eq!(built.into_text(), "ab");
    }

    #[test]
    fn limited_text_stops_writing_past_the_limit() {
        let mut text = LimitedText::default();
        text.write_str(&BuildLimitFixture::text_of(BuildLimit::TEXT_BYTES))
            .expect("it fits");
        assert!(text.write_str("c").is_err());
        assert!(BuildLimitFixture::limit().check(text).is_err());
    }

    #[test]
    fn a_planned_list_past_the_limit_is_refused() {
        let limit = BuildLimitFixture::limit();
        assert!(limit.check(PlannedList(BuildLimit::LIST_SIZE)).is_ok());
        assert!(limit.check(PlannedList(BuildLimit::LIST_SIZE + 1)).is_err());
    }

    #[test]
    fn module_notes_past_their_limit_are_refused() {
        let limit = BuildLimitFixture::limit();
        assert!(limit.check(HeldNotes(BuildLimit::MODULE_NOTES)).is_ok());
        assert!(
            limit
                .check(HeldNotes(BuildLimit::MODULE_NOTES + 1))
                .is_err()
        );
    }
}

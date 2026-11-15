use super::lint_level::LintLevel;
use crate::diagnostic::DiagnosticCode;
use crate::source::Span;

#[derive(Debug, Default)]
pub struct LintTable {
    entries: Vec<LintEntry>,
}

#[derive(Debug)]
struct LintEntry {
    code: DiagnosticCode,
    level: LintLevel,
    span: Span,
}

impl LintTable {
    pub fn levels(&self) -> impl Iterator<Item = (&DiagnosticCode, LintLevel)> {
        self.entries.iter().map(|entry| (&entry.code, entry.level))
    }

    pub(super) fn add(&mut self, code: DiagnosticCode, level: LintLevel, span: Span) {
        self.entries.push(LintEntry { code, level, span });
    }

    pub(super) fn span_of(&self, code: &DiagnosticCode) -> Option<Span> {
        self.entries
            .iter()
            .find(|entry| entry.code == *code)
            .map(|entry| entry.span)
    }
}

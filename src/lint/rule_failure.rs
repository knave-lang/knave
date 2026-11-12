use crate::diagnostic::{Diagnostic, DiagnosticRenderer, Diagnostics};
use crate::source::SourceMap;

#[derive(Debug)]
pub struct RuleFailure {
    report: String,
}

impl RuleFailure {
    pub fn of_all(diagnostics: &Diagnostics, sources: &SourceMap) -> Self {
        Self {
            report: DiagnosticRenderer::plain(sources).render_all(diagnostics),
        }
    }

    pub fn of(diagnostic: Diagnostic, sources: &SourceMap) -> Self {
        let mut diagnostics = Diagnostics::default();
        diagnostics.push(diagnostic);
        Self::of_all(&diagnostics, sources)
    }

    pub fn report(&self) -> &str {
        &self.report
    }
}

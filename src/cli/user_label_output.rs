use std::rc::Rc;

use super::diagnostic_output::DiagnosticOutput;
use crate::diagnostic::Diagnostics;
use crate::source::{SourceId, SourceMap};

pub struct UserLabelOutput {
    inner: Rc<dyn DiagnosticOutput>,
    generated: Vec<&'static str>,
}

impl UserLabelOutput {
    pub fn new(inner: Rc<dyn DiagnosticOutput>, generated: Vec<&'static str>) -> Self {
        Self { inner, generated }
    }

    fn without_generated_labels(
        &self,
        sources: &SourceMap,
        diagnostics: &Diagnostics,
    ) -> Diagnostics {
        diagnostics.clone().map(|diagnostic| {
            let generated: Vec<SourceId> = diagnostic
                .labels()
                .map(|label| label.span().source())
                .filter(|source| self.generated.contains(&sources.file(*source).name()))
                .collect();
            generated
                .into_iter()
                .fold(diagnostic, |diagnostic, source| {
                    diagnostic.without_labels_in(source)
                })
        })
    }
}

impl DiagnosticOutput for UserLabelOutput {
    fn write(&self, sources: &SourceMap, diagnostics: &Diagnostics) {
        self.inner.write(
            sources,
            &self.without_generated_labels(sources, diagnostics),
        );
    }

    fn write_note(&self, sources: &SourceMap, message: &str) {
        self.inner.write_note(sources, message);
    }
}

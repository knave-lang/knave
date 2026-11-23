use std::rc::Rc;

use super::diagnostic_output::{DiagnosticOutput, MessageOutput};
use crate::diagnostic::Diagnostics;
use crate::source::SourceMap;

pub struct ErrorsOnlyOutput {
    inner: Rc<dyn DiagnosticOutput>,
}

impl ErrorsOnlyOutput {
    pub fn new(inner: Rc<dyn DiagnosticOutput>) -> Self {
        Self { inner }
    }

    fn errors(diagnostics: &Diagnostics) -> Diagnostics {
        let mut errors = diagnostics.clone();
        errors.retain(|diagnostic| diagnostic.severity().is_error());
        errors
    }
}

impl MessageOutput for ErrorsOnlyOutput {}

impl DiagnosticOutput for ErrorsOnlyOutput {
    fn write(&self, sources: &SourceMap, diagnostics: &Diagnostics) {
        let errors = Self::errors(diagnostics);
        if !errors.is_empty() {
            self.inner.write(sources, &errors);
        }
    }

    fn write_note(&self, sources: &SourceMap, message: &str) {
        self.inner.write_note(sources, message);
    }

    fn write_after_earlier(&self, sources: &SourceMap, diagnostics: &Diagnostics) {
        let errors = Self::errors(diagnostics);
        if !errors.is_empty() {
            self.inner.write_after_earlier(sources, &errors);
        }
    }
}

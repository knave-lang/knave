use std::cell::Cell;
use std::rc::Rc;

use super::diagnostic_output::{DiagnosticOutput, MessageOutput};
use crate::diagnostic::Diagnostics;
use crate::source::SourceMap;

pub struct SeparatedBatches {
    inner: Rc<dyn DiagnosticOutput>,
    wrote_a_batch: Cell<bool>,
}

impl SeparatedBatches {
    pub fn new(inner: Rc<dyn DiagnosticOutput>) -> Self {
        Self {
            inner,
            wrote_a_batch: Cell::new(false),
        }
    }
}

impl MessageOutput for SeparatedBatches {}

impl DiagnosticOutput for SeparatedBatches {
    fn write(&self, sources: &SourceMap, diagnostics: &Diagnostics) {
        match self.wrote_a_batch.replace(true) {
            true => self.inner.write_after_earlier(sources, diagnostics),
            false => self.inner.write(sources, diagnostics),
        }
    }

    fn write_note(&self, sources: &SourceMap, message: &str) {
        self.inner.write_note(sources, message);
    }
}

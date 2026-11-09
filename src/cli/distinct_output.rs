use std::cell::RefCell;
use std::rc::Rc;

use rustc_hash::FxHashSet;

use super::diagnostic_output::DiagnosticOutput;
use crate::diagnostic::{DiagnosticRenderer, Diagnostics};
use crate::source::SourceMap;

pub struct DistinctOutput {
    inner: Rc<dyn DiagnosticOutput>,
    written: RefCell<FxHashSet<String>>,
}

impl DistinctOutput {
    pub fn new(inner: Rc<dyn DiagnosticOutput>) -> Self {
        Self {
            inner,
            written: RefCell::default(),
        }
    }
}

impl DiagnosticOutput for DistinctOutput {
    fn write(&self, sources: &SourceMap, diagnostics: &Diagnostics) {
        let renderer = DiagnosticRenderer::plain(sources);
        let mut written = self.written.borrow_mut();
        let mut unwritten = diagnostics.clone();
        unwritten.retain(|diagnostic| written.insert(renderer.render(diagnostic)));
        if !unwritten.is_empty() {
            self.inner.write(sources, &unwritten);
        }
    }

    fn write_note(&self, sources: &SourceMap, message: &str) {
        self.inner.write_note(sources, message);
    }
}

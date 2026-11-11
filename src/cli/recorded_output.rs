use std::cell::RefCell;

use super::diagnostic_output::{DiagnosticOutput, MessageOutput};
use crate::diagnostic::Diagnostics;
use crate::source::SourceMap;

#[derive(Default)]
pub struct RecordedOutput {
    writes: RefCell<Vec<String>>,
}

impl RecordedOutput {
    pub fn writes(&self) -> Vec<String> {
        self.writes.borrow().clone()
    }

    fn record(&self, write: String) {
        self.writes.borrow_mut().push(write);
    }
}

impl DiagnosticOutput for RecordedOutput {
    fn write(&self, _: &SourceMap, diagnostics: &Diagnostics) {
        self.record(format!("{} diagnostics", diagnostics.len()));
    }

    fn write_note(&self, _: &SourceMap, message: &str) {
        self.record(format!("note: {message}"));
    }
}

impl MessageOutput for RecordedOutput {
    fn write_error(&self, message: &str) {
        self.record(format!("error: {message}"));
    }
}

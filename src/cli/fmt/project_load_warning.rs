use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::cli::diagnostic_output::DiagnosticOutput;
use crate::cli::standard_error::StandardError;
use crate::diagnostic::Diagnostics;
use crate::source::SourceMap;

pub struct ProjectLoadWarning {
    formatted_file: PathBuf,
    output: Rc<dyn DiagnosticOutput>,
    warned: Cell<bool>,
}

impl ProjectLoadWarning {
    pub fn new(formatted_file: &Path, output: Rc<dyn DiagnosticOutput>) -> Self {
        Self {
            formatted_file: formatted_file.to_path_buf(),
            output,
            warned: Cell::new(false),
        }
    }

    pub fn warn_once(&self) {
        if self.warned.replace(true) {
            return;
        }
        StandardError::write_line(format_args!(
            "warning: cannot load the project around `{}` with its libraries, so `fmt` reads shapes only beside each file and in its parent directories",
            self.formatted_file.display()
        ));
    }
}

impl DiagnosticOutput for ProjectLoadWarning {
    fn write(&self, sources: &SourceMap, diagnostics: &Diagnostics) {
        self.warn_once();
        self.output.write(sources, diagnostics);
    }

    fn write_note(&self, sources: &SourceMap, message: &str) {
        self.warn_once();
        self.output.write_note(sources, message);
    }

    fn write_error(&self, message: &str) {
        self.warn_once();
        self.output.write_error(message);
    }
}

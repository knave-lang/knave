use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::cli::diagnostic_output::{DiagnosticOutput, MessageOutput};
use crate::cli::fmt::formatted_files::FormattedFiles;
use crate::cli::standard_error::StandardError;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::source::SourceMap;

pub struct ProjectLoadWarning {
    looked_up_file: PathBuf,
    output: Rc<dyn DiagnosticOutput>,
    formatted_files: Rc<FormattedFiles>,
    warned: Cell<bool>,
}

impl ProjectLoadWarning {
    pub fn new(
        looked_up_file: &Path,
        output: Rc<dyn DiagnosticOutput>,
        formatted_files: Rc<FormattedFiles>,
    ) -> Self {
        Self {
            looked_up_file: looked_up_file.to_path_buf(),
            output,
            formatted_files,
            warned: Cell::new(false),
        }
    }

    pub fn warn_once(&self) {
        if self.warned.replace(true) {
            return;
        }
        StandardError::write_line(format_args!(
            "warning: cannot load the project around `{}` with its libraries, so `fmt` reads shapes only beside each file and in its parent directories",
            self.looked_up_file.display()
        ));
    }
}

impl DiagnosticOutput for ProjectLoadWarning {
    fn write(&self, sources: &SourceMap, diagnostics: &Diagnostics) {
        self.warn_once();
        let warnings = diagnostics.clone().map(Diagnostic::into_warning);
        let warnings = self.formatted_files.without_unreadable(sources, warnings);
        if !warnings.is_empty() {
            self.output.write(sources, &warnings);
        }
    }

    fn write_note(&self, sources: &SourceMap, message: &str) {
        self.output.write_note(sources, message);
    }
}

impl MessageOutput for ProjectLoadWarning {
    fn write_error(&self, message: &str) {
        self.warn_once();
        self.output.write_warning(message);
    }

    fn write_unreadable(&self, path: &Path, message: &str) {
        if self.formatted_files.includes(path) {
            self.warn_once();
        } else {
            self.write_error(message);
        }
    }
}

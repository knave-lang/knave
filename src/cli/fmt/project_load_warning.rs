use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::cli::diagnostic_output::DiagnosticOutput;
use crate::cli::standard_error::StandardError;
use crate::cli::terminal::Terminal;
use crate::diagnostic::Diagnostics;
use crate::language::Language;
use crate::source::SourceMap;

pub struct ProjectLoadWarning {
    project_file: PathBuf,
    output: Rc<dyn DiagnosticOutput>,
    warned: Cell<bool>,
}

impl ProjectLoadWarning {
    pub fn new(root: &Path, output: Rc<dyn DiagnosticOutput>) -> Self {
        Self {
            project_file: Self::shown(&root.join(Language::project_file_name())),
            output,
            warned: Cell::new(false),
        }
    }

    fn shown(path: &Path) -> PathBuf {
        fs::canonicalize(Terminal::WORKING_DIRECTORY)
            .ok()
            .and_then(|working_directory| path.strip_prefix(working_directory).ok())
            .unwrap_or(path)
            .to_path_buf()
    }

    pub fn warn_once(&self) {
        if self.warned.replace(true) {
            return;
        }
        StandardError::write_line(format_args!(
            "warning: cannot load `{}` with its libraries, so `fmt` reads shapes only beside each file and in its parent directories",
            self.project_file.display()
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

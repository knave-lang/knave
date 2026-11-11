use std::path::Path;

use crate::cli::diagnostic_output::DiagnosticOutput;
use crate::cli::diagnostic_output::UnreadableFile;
use crate::cli::exit_status::ExitStatus;
use crate::diagnostic::Diagnostics;
use crate::modules::ModuleSources;
use crate::project::{Project, ProjectReader};
use crate::reader::{Form, Reader};
use crate::source::{SourceId, SourceMap, Span, TextPosition};

pub struct SourceLoader<'a> {
    sources: &'a mut SourceMap,
    output: &'a dyn DiagnosticOutput,
}

impl<'a> SourceLoader<'a> {
    pub fn new(sources: &'a mut SourceMap, output: &'a dyn DiagnosticOutput) -> Self {
        Self { sources, output }
    }

    pub fn load(&mut self, path: &Path) -> Result<SourceId, ExitStatus> {
        match ModuleSources::read_file(path) {
            Ok(text) => Ok(self.add(path, text)),
            Err(error) => Err(UnreadableFile::new(path, &error).report()),
        }
    }

    pub fn add(&mut self, path: &Path, text: String) -> SourceId {
        self.sources.add(path.display().to_string(), text)
    }

    pub fn add_named(&mut self, name: &str, text: &str) -> SourceId {
        self.sources.add(name, text)
    }

    pub fn add_inside(
        &mut self,
        enclosing: SourceId,
        position: TextPosition,
        text: &str,
    ) -> Option<SourceId> {
        self.sources.add_inside(enclosing, position, text)
    }

    pub fn read(&self, source: SourceId) -> Result<Vec<Form>, ExitStatus> {
        Reader::new(self.sources.file(source))
            .read()
            .map_err(|diagnostics| self.report(&diagnostics))
    }

    pub fn read_project(&self, source: SourceId) -> Result<Project, ExitStatus> {
        let forms = self.read(source)?;
        let whole_file = Span::new(source, 0..self.sources.file(source).text().len());
        ProjectReader::read(&forms, whole_file).map_err(|diagnostics| self.report(&diagnostics))
    }

    fn report(&self, diagnostics: &Diagnostics) -> ExitStatus {
        self.output.report(self.sources, diagnostics)
    }
}

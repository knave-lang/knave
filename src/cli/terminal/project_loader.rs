use std::io;
use std::path::Path;

use crate::cli::diagnostic_output::DiagnosticOutput;
use crate::cli::embedded_prelude::EmbeddedPrelude;
use crate::cli::exit_status::ExitStatus;
use crate::cli::unreadable_file::UnreadableFile;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::modules::ModuleSources;
use crate::project::{Project, ProjectReader};
use crate::reader::{Form, Reader};
use crate::source::{SourceId, SourceMap, Span};

pub struct ProjectLoader<'a> {
    sources: &'a mut SourceMap,
    output: &'a dyn DiagnosticOutput,
}

impl<'a> ProjectLoader<'a> {
    pub fn new(sources: &'a mut SourceMap, output: &'a dyn DiagnosticOutput) -> Self {
        Self { sources, output }
    }

    pub fn load(&mut self, project_file: &Path) -> Result<Project, ExitStatus> {
        let source = self.added_or_reported(project_file)?;
        self.read(source)
    }

    pub fn read(&mut self, source: SourceId) -> Result<Project, ExitStatus> {
        self.forms(source)
            .and_then(|forms| Self::reader().read(&forms, self.whole_file(source)))
            .map_err(|diagnostics| self.report(&diagnostics))
    }

    pub fn read_quietly(&mut self, project_file: &Path) -> Option<Project> {
        let source = self.added(project_file).ok()?;
        let forms = self.forms(source).ok()?;
        Self::reader().read(&forms, self.whole_file(source)).ok()
    }

    pub fn version_refusal(&mut self, project_file: &Path) -> Option<Diagnostic> {
        let source = self.added(project_file).ok()?;
        let forms = self.forms(source).ok()?;
        Self::reader().refusal(&forms, self.whole_file(source))
    }

    fn added_or_reported(&mut self, project_file: &Path) -> Result<SourceId, ExitStatus> {
        self.added(project_file)
            .map_err(|error| UnreadableFile::new(project_file, &error).report_to(self.output))
    }

    fn added(&mut self, project_file: &Path) -> io::Result<SourceId> {
        let text = ModuleSources::read_file(project_file)?;
        Ok(self.sources.add(project_file.display().to_string(), text))
    }

    fn forms(&self, source: SourceId) -> Result<Vec<Form>, Diagnostics> {
        Reader::new(self.sources.file(source)).read()
    }

    fn report(&self, diagnostics: &Diagnostics) -> ExitStatus {
        self.output.report(self.sources, diagnostics)
    }

    fn reader() -> ProjectReader<'static> {
        ProjectReader::new(EmbeddedPrelude::project_vocabulary())
    }

    fn whole_file(&self, source: SourceId) -> Span {
        Span::new(source, 0..self.sources.file(source).text().len())
    }
}

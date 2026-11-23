use std::path::Path;

use super::diagnostic_output::StandardErrorMessages;
use super::embedded_prelude::EmbeddedPrelude;
use super::exit_status::ExitStatus;
use super::unreadable_file::UnreadableFile;
use crate::diagnostic::Diagnostics;
use crate::expand::ExpansionSession;
use crate::modules::ModuleSources;
use crate::project::{ExportedTask, ProjectReader};
use crate::reader::{Form, FormKind, Reader, Symbol};
use crate::source::{SourceId, SourceMap, Span};

pub struct ExportedTaskNames {
    name: String,
    text: String,
    count: usize,
}

impl ExportedTaskNames {
    const KEYWORD: &'static str = ":exported-task";

    pub fn read(project_file: &Path) -> Result<Self, ExitStatus> {
        let text = ModuleSources::read_file(project_file).map_err(|error| {
            UnreadableFile::new(project_file, &error).report_to(&StandardErrorMessages)
        })?;
        let mut sources = SourceMap::default();
        let source = sources.add(project_file.display().to_string(), text.clone());
        let count = Self::tasks_in(&sources, source, &text).map_or(0, |tasks| tasks.len());
        Ok(Self {
            name: project_file.display().to_string(),
            text,
            count,
        })
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn argument(
        &self,
        session: &mut ExpansionSession<'_>,
        index: usize,
    ) -> Result<Vec<Form>, Diagnostics> {
        let source = session.add_source(&self.name, &self.text);
        let whole_file = Span::new(source, 0..self.text.len());
        let project = ProjectReader::new(EmbeddedPrelude::project_vocabulary())
            .read(&session.read(source)?, whole_file)?;
        Ok(project
            .exported_tasks
            .into_iter()
            .flatten()
            .nth(index)
            .map(|task| {
                vec![
                    Self::symbol(Self::KEYWORD, task.span),
                    Self::symbol(&task.name, task.span),
                ]
            })
            .unwrap_or_default())
    }

    fn tasks_in(sources: &SourceMap, source: SourceId, text: &str) -> Option<Vec<ExportedTask>> {
        let forms = Reader::new(sources.file(source)).read().ok()?;
        let whole_file = Span::new(source, 0..text.len());
        ProjectReader::new(EmbeddedPrelude::project_vocabulary())
            .read(&forms, whole_file)
            .ok()?
            .exported_tasks
    }

    fn symbol(name: &str, span: Span) -> Form {
        Form::new(FormKind::Symbol(Symbol::new(name)), span)
    }
}

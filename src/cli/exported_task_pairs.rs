use std::path::Path;

use super::diagnostic_output::StandardErrorMessages;
use super::embedded_prelude::EmbeddedPrelude;
use super::exit_status::ExitStatus;
use super::unreadable_file::UnreadableFile;
use crate::diagnostic::Diagnostics;
use crate::expand::ExpansionSession;
use crate::modules::ModuleSources;
use crate::project::ProjectReader;
use crate::reader::{Form, FormKind, Symbol};
use crate::source::Span;

pub struct ExportedTaskPairs {
    name: String,
    text: String,
}

impl ExportedTaskPairs {
    const KEYWORD: &'static str = ":exported-task";

    pub fn read(project_file: &Path) -> Result<Self, ExitStatus> {
        let text = ModuleSources::read_file(project_file).map_err(|error| {
            UnreadableFile::new(project_file, &error).report_to(&StandardErrorMessages)
        })?;
        Ok(Self {
            name: project_file.display().to_string(),
            text,
        })
    }

    pub fn forms(&self, session: &mut ExpansionSession<'_>) -> Result<Vec<Form>, Diagnostics> {
        let source = session.add_source(&self.name, &self.text);
        let whole_file = Span::new(source, 0..self.text.len());
        let project = ProjectReader::new(EmbeddedPrelude::project_vocabulary())
            .read(&session.read(source)?, whole_file)?;
        Ok(project
            .exported_tasks
            .into_iter()
            .flat_map(|task| {
                [
                    Self::symbol(Self::KEYWORD, task.span),
                    Self::symbol(&task.name, task.span),
                ]
            })
            .collect())
    }

    fn symbol(name: &str, span: Span) -> Form {
        Form::new(FormKind::Symbol(Symbol::new(name)), span)
    }
}

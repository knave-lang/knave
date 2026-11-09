use super::exit_status::ExitStatus;
use super::project_file_edit::ProjectFileEdit;
use super::standard_error::StandardError;
use super::terminal::Terminal;
use crate::project::Section;

pub struct RemoveCommand {
    name: String,
}

impl RemoveCommand {
    const COMMAND: &str = "remove";

    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        match self.remove(terminal) {
            Ok(()) => ExitStatus::Success,
            Err(status) => status,
        }
    }

    fn remove(&self, terminal: &Terminal) -> Result<(), ExitStatus> {
        let edit = ProjectFileEdit::of_working_directory(terminal, Self::COMMAND)?;
        let removed = edit.apply(terminal, |editor| {
            editor.without_crate(Section::Dependencies, &self.name)
        })?;
        if removed {
            return Ok(());
        }
        StandardError::write_line(format_args!(
            "error: the project's `dependencies` do not list `{}`",
            self.name
        ));
        Err(ExitStatus::UsageError)
    }
}

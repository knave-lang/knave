use std::fs;
use std::path::Path;

use super::exit_status::ExitStatus;
use super::new_components::NewComponents;
use super::owned_file::OwnedFile;
use super::program_file::ProgramFile;
use super::project_file_edit::ProjectFileEdit;
use super::standard_error::StandardError;
use super::terminal::Terminal;
use crate::language::Language;

pub struct UpgradeCommand;

impl UpgradeCommand {
    const COMMAND: &str = "upgrade";

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        match Self::upgrade(terminal) {
            Ok(()) => ExitStatus::Success,
            Err(status) => status,
        }
    }

    fn upgrade(terminal: &mut Terminal) -> Result<(), ExitStatus> {
        let edit = ProjectFileEdit::of_root_file(terminal, Self::COMMAND)?;
        let mut changed = Self::record_version(&edit, terminal)?;
        for file in NewComponents::every()
            .iter()
            .flat_map(|component| component.owned_files())
        {
            changed |= Self::refresh(edit.root(), &file)?;
        }
        if !changed {
            Self::report_current();
        }
        Ok(())
    }

    fn record_version(edit: &ProjectFileEdit, terminal: &Terminal) -> Result<bool, ExitStatus> {
        let version = format!("\"{}\"", Language::VERSION);
        let recorded = edit.apply(terminal, |editor| {
            editor.with_field(Language::NAME, &version)
        })?;
        if recorded {
            StandardError::write_line(format_args!(
                "recorded Knave {} in `{}`",
                Language::VERSION,
                Language::project_file_name()
            ));
        }
        Ok(recorded)
    }

    fn refresh(root: &Path, file: &OwnedFile) -> Result<bool, ExitStatus> {
        let path = root.join(file.path());
        let Ok(written) = fs::read_to_string(&path) else {
            return Ok(false);
        };
        if written == file.current_text() {
            Ok(false)
        } else {
            Self::rewrite(&path, file)
        }
    }

    fn rewrite(path: &Path, file: &OwnedFile) -> Result<bool, ExitStatus> {
        fs::write(path, file.current_text())
            .map_err(|error| ProgramFile::report_unwritable(path, &error))?;
        StandardError::write_line(format_args!("rewrote `{}`", file.path().display()));
        Ok(true)
    }

    fn report_current() {
        StandardError::write_line(format_args!(
            "note: the project already uses Knave {}, the version of this `{}`\nhelp: to install a newer Knave, run `cargo install {} --locked`",
            Language::VERSION,
            Language::NAME,
            Language::NAME
        ));
    }
}

use std::fs;
use std::path::Path;

use similar::TextDiff;

use super::exit_status::ExitStatus;
use super::new_components::NewComponents;
use super::owned_file::{OwnedFile, OwnedFileState};
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
        let edit = ProjectFileEdit::to_upgrade(terminal, Self::COMMAND)?;
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
        match file.state(&written) {
            OwnedFileState::Current => Ok(false),
            OwnedFileState::WrittenByEarlierKnave => Self::rewrite(&path, file),
            OwnedFileState::ChangedByUser => {
                Self::report_kept(file, &written);
                Ok(true)
            }
        }
    }

    fn rewrite(path: &Path, file: &OwnedFile) -> Result<bool, ExitStatus> {
        fs::write(path, file.current_text())
            .map_err(|error| ProgramFile::report_unwritable(path, &error))?;
        StandardError::write_line(format_args!("rewrote `{}`", file.path().display()));
        Ok(true)
    }

    fn report_kept(file: &OwnedFile, written: &str) {
        let shown = file.path().display().to_string();
        let version = Language::VERSION;
        let current = file.current_text();
        let diff = TextDiff::from_lines(written, &current)
            .unified_diff()
            .header(&shown, &format!("{shown} (Knave {version})"))
            .to_string();
        StandardError::write_line(format_args!(
            "warning: kept `{shown}`, because it differs from every text Knave wrote\nhelp: the lines marked `-` are yours and the lines marked `+` are Knave {version}'s: merge them by hand\n{diff}"
        ));
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

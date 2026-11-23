use std::fs;
use std::path::Path;

use super::exit_status::ExitStatus;
use super::github_linguist::GithubLinguist;
use super::new_components::NewComponents;
use super::owned_file::OwnedFile;
use super::program_file::ProgramFile;
use super::project_component::ProjectComponent;
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
        for component in NewComponents::every() {
            changed |= Self::refresh_component(edit.root(), component.as_ref())?;
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

    fn refresh_component(
        root: &Path,
        component: &dyn ProjectComponent,
    ) -> Result<bool, ExitStatus> {
        let mut changed = false;
        for file in component.owned_files() {
            changed |= Self::refresh(root, &file)?;
        }
        if Self::shows(root, component) {
            for line in component.owned_lines() {
                changed |= Self::refresh_line(root, &line)?;
            }
        }
        Ok(changed)
    }

    fn shows(root: &Path, component: &dyn ProjectComponent) -> bool {
        component
            .owned_files()
            .iter()
            .any(|file| root.join(file.path()).exists())
    }

    fn refresh_line(root: &Path, line: &GithubLinguist) -> Result<bool, ExitStatus> {
        let path = root.join(GithubLinguist::PATH);
        let Ok(written) = fs::read_to_string(&path) else {
            return Ok(false);
        };
        let upgraded = line.upgraded(&written);
        if upgraded == written {
            return Ok(false);
        }
        fs::write(&path, upgraded)
            .map_err(|error| ProgramFile::report_unwritable(&path, &error))?;
        StandardError::write_line(format_args!(
            "rewrote the `{}` line of `{}`",
            GithubLinguist::PATTERN,
            GithubLinguist::PATH
        ));
        Ok(true)
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

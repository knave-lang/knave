use std::path::Path;

use super::exit_status::ExitStatus;
use super::new_components::NewComponents;
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
        let components = NewComponents::every();
        let mut changed = Self::record_version(&edit, terminal)?;
        for component in components
            .iter()
            .filter(|component| component.shows(edit.root()))
        {
            changed |= Self::bring_component_current(edit.root(), component.as_ref())?;
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

    fn bring_component_current(
        root: &Path,
        component: &dyn ProjectComponent,
    ) -> Result<bool, ExitStatus> {
        let mut changed = false;
        for file in component.owned_files() {
            changed |= file.bring_current(root)?;
        }
        for line in component.owned_lines() {
            changed |= line.bring_current(root)?;
        }
        Ok(changed)
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

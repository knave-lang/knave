use std::path::Path;

use super::announced_write::AnnouncedWrite;
use super::exit_status::ExitStatus;
use super::new_components::NewComponents;
use super::pending_writes::PendingWrites;
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
        let writes = Self::pending_writes(edit.root(), &components)?;
        let recorded = Self::record_version(&edit, terminal)?;
        Self::apply(&writes)?;
        if !recorded && writes.is_empty() {
            Self::report_current();
        }
        Ok(())
    }

    fn apply(writes: &[AnnouncedWrite]) -> Result<(), ExitStatus> {
        writes.iter().try_for_each(AnnouncedWrite::apply)
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

    fn pending_writes(
        root: &Path,
        components: &[Box<dyn ProjectComponent>],
    ) -> Result<Vec<AnnouncedWrite>, ExitStatus> {
        components
            .iter()
            .filter(|component| component.shows(root))
            .map(|component| component.pending_writes(root))
            .collect::<PendingWrites>()
            .into_writes()
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

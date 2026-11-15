use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::project_inspection::ProjectInspection;
use super::terminal::Terminal;
use crate::ownership::EnforcedOwnership;

pub struct CheckCommand {
    entry: EntryArgument,
}

impl CheckCommand {
    pub fn new(entry: EntryArgument) -> Self {
        Self { entry }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        let inspected = ProjectInspection::new(self.entry).run(
            terminal,
            &|report, warnings| report.print(warnings),
            |terminal, file, _| terminal.check_file::<EnforcedOwnership>(file).map(drop),
        );
        ExitStatus::of(inspected)
    }
}

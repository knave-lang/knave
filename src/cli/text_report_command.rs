use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::standard_output::StandardOutput;
use super::terminal::Terminal;
use crate::ownership::{TextDecisions, TextReportOwnership};

pub struct TextReportCommand {
    entry: EntryArgument,
}

impl TextReportCommand {
    pub fn new(entry: EntryArgument) -> Self {
        Self { entry }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        let checked = self
            .entry
            .locate(terminal)
            .and_then(|program| terminal.check_file::<TextReportOwnership>(program.file()));
        match checked {
            Ok(program) => {
                let report = TextDecisions::of(&program).report(terminal.source_report().sources());
                StandardOutput::locked().write(report)
            }
            Err(status) => status,
        }
    }
}

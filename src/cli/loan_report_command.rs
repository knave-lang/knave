use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::standard_output::StandardOutput;
use super::terminal::Terminal;
use crate::ownership::{LoanFacts, WaivedOwnership};

pub struct LoanReportCommand {
    entry: EntryArgument,
}

impl LoanReportCommand {
    pub fn new(entry: EntryArgument) -> Self {
        Self { entry }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        let checked = self
            .entry
            .locate(terminal)
            .and_then(|program| terminal.check_file::<WaivedOwnership>(program.file()));
        match checked {
            Ok(program) => {
                let report = LoanFacts::of(&program).report(terminal.source_report().sources());
                StandardOutput::locked().write(report)
            }
            Err(status) => status,
        }
    }
}

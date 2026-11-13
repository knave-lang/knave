use clap::Args;

use super::check::CheckCommand;
use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::loan_report_command::LoanReportCommand;
use super::terminal::Terminal;

#[derive(Args)]
pub struct CheckArguments {
    #[command(flatten)]
    entry: EntryArgument,
    #[arg(long, hide = true)]
    print_loans: bool,
}

impl CheckArguments {
    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        if self.print_loans {
            LoanReportCommand::new(self.entry).run(terminal)
        } else {
            CheckCommand::new(self.entry).run(terminal)
        }
    }
}

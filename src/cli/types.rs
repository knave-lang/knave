use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::standard_output::StandardOutput;
use super::terminal::Terminal;
use crate::ownership::EnforcedOwnership;

pub struct TypesCommand {
    entry: EntryArgument,
}

impl TypesCommand {
    pub fn new(entry: EntryArgument) -> Self {
        Self { entry }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        match self
            .entry
            .locate(terminal)
            .and_then(|program| terminal.check_file::<EnforcedOwnership>(program.file()))
        {
            Ok(program) => StandardOutput::locked().write(program.entry_signatures()),
            Err(status) => status,
        }
    }
}

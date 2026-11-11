use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::terminal::Terminal;

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
            .and_then(|program| terminal.check(program.file()))
        {
            Ok(program) => terminal.write_output(program.entry_signatures()),
            Err(status) => status,
        }
    }
}

use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::terminal::Terminal;

pub struct EmitCommand {
    entry: EntryArgument,
}

impl EmitCommand {
    pub fn new(entry: EntryArgument) -> Self {
        Self { entry }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        match self
            .entry
            .locate(terminal)
            .and_then(|program| terminal.emit(program.file()))
        {
            Ok(emitted) => terminal.write_output(&emitted.code),
            Err(status) => status,
        }
    }
}

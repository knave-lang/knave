use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::terminal::Terminal;
use crate::core::CorePrinter;

pub struct ExpandCommand {
    entry: EntryArgument,
}

impl ExpandCommand {
    pub fn new(entry: EntryArgument) -> Self {
        Self { entry }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        match self
            .entry
            .locate(terminal)
            .and_then(|program| terminal.expand(program.file()))
        {
            Ok(program) => terminal
                .write_output(CorePrinter::new(&program.bindings).module(program.entry_module())),
            Err(status) => status,
        }
    }
}

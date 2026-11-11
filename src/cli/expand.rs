use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::standard_output::StandardOutput;
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
        match self.entry.locate(terminal).and_then(|program| {
            let entry = terminal.sources().load(program.file().path())?;
            terminal.expansion().expand(program.file(), entry)
        }) {
            Ok(program) => StandardOutput::locked()
                .write(CorePrinter::new(&program.bindings).module(program.entry_module())),
            Err(status) => status,
        }
    }
}

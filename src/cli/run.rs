use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::finished_program::FinishedProgram;
use super::terminal::{BuildOptions, Terminal};

pub struct RunCommand {
    entry: EntryArgument,
    options: BuildOptions,
    arguments: Vec<String>,
}

impl RunCommand {
    pub fn new(entry: EntryArgument, options: BuildOptions, arguments: Vec<String>) -> Self {
        Self {
            entry,
            options,
            arguments,
        }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        match self
            .entry
            .locate(terminal)
            .and_then(|program| terminal.build(&program, &self.options))
        {
            Ok(binary) => FinishedProgram::run_with(&binary, &self.arguments).status(),
            Err(status) => status,
        }
    }
}

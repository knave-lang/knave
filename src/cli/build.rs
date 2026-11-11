use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::terminal::{BuildOptions, Terminal};

pub struct BuildCommand {
    entry: EntryArgument,
    options: BuildOptions,
}

impl BuildCommand {
    pub fn new(entry: EntryArgument, options: BuildOptions) -> Self {
        Self { entry, options }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        match self
            .entry
            .locate(terminal)
            .and_then(|program| terminal.build(&program, &self.options))
        {
            Ok(_) => ExitStatus::Success,
            Err(status) => status,
        }
    }
}

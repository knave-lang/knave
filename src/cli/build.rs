use super::build_options::BuildOptions;
use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::program_binary::ProgramBinary;
use super::terminal::Terminal;

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
            .and_then(|program| ProgramBinary::new(&program, &self.options).build(terminal))
        {
            Ok(_) => ExitStatus::Success,
            Err(status) => status,
        }
    }
}

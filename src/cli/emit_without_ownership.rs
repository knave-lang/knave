use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::program_file::ProgramFile;
use super::standard_output::StandardOutput;
use super::terminal::Terminal;
use crate::emit::EmittedCrate;

pub struct EmitWithoutOwnershipCommand {
    entry: EntryArgument,
}

impl EmitWithoutOwnershipCommand {
    pub fn new(entry: EntryArgument) -> Self {
        Self { entry }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        match self
            .entry
            .locate(terminal)
            .and_then(|program| Self::emit(terminal, program.file()))
        {
            Ok(emitted) => StandardOutput::locked().write(&emitted.code),
            Err(status) => status,
        }
    }

    fn emit(terminal: &mut Terminal, file: &ProgramFile) -> Result<EmittedCrate, ExitStatus> {
        let entry = terminal.sources().load(file.path())?;
        let program = terminal.expansion().expand(file, entry)?;
        let typed = terminal.checking().check_without_ownership(&program)?;
        terminal.emission()?.emit(&typed)
    }
}

use std::marker::PhantomData;

use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::standard_output::StandardOutput;
use super::terminal::Terminal;
use crate::ownership::OwnershipGate;

pub struct EmitCommand<Gate: OwnershipGate> {
    entry: EntryArgument,
    gate: PhantomData<Gate>,
}

impl<Gate: OwnershipGate> EmitCommand<Gate> {
    pub fn new(entry: EntryArgument) -> Self {
        Self {
            entry,
            gate: PhantomData,
        }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        match self
            .entry
            .locate(terminal)
            .and_then(|program| terminal.emit_file::<Gate>(program.file()))
        {
            Ok(emitted) => StandardOutput::locked().write(&emitted.code),
            Err(status) => status,
        }
    }
}

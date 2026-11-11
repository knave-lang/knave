use super::exit_status::ExitStatus;
use super::interruption::Interruption;
use super::standard_error::StandardError;
use crate::build::{EmittedFormatting, FormattingError};
use crate::emit::EmittedCrate;

pub struct InterruptibleFormatting<'a> {
    formatting: EmittedFormatting,
    interruption: &'a Interruption,
}

impl<'a> InterruptibleFormatting<'a> {
    pub fn new(formatting: EmittedFormatting, interruption: &'a Interruption) -> Self {
        Self {
            formatting,
            interruption,
        }
    }

    pub fn format_or_keep_unformatted(
        &self,
        emitted: EmittedCrate,
    ) -> Result<EmittedCrate, ExitStatus> {
        match self.formatting.apply(&emitted) {
            Ok(formatted) => Ok(formatted),
            Err(FormattingError::Interrupted) => Err(ExitStatus::Interrupted),
            Err(_) if self.interruption.caused_a_failure() => Err(ExitStatus::Interrupted),
            Err(error) => {
                StandardError::write_line(format_args!("note: {error}; writing unformatted Rust"));
                Ok(emitted)
            }
        }
    }
}

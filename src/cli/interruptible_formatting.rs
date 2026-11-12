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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::InterruptibleFormatting;
    use crate::build::EmittedFormatting;
    use crate::cli::fake_programs::FakePrograms;
    use crate::cli::fake_stages::FakeStages;
    use crate::cli::interruption::Interruption;

    struct FormattingFixture {
        interruption: Interruption,
    }

    impl FormattingFixture {
        fn new() -> Self {
            Self {
                interruption: Interruption::default(),
            }
        }

        fn formatting(&self, limit: Duration) -> InterruptibleFormatting<'_> {
            InterruptibleFormatting::new(EmittedFormatting::within(limit), &self.interruption)
        }
    }

    #[test]
    fn formats_the_emitted_rust() {
        let fixture = FormattingFixture::new();
        let written = fixture
            .formatting(EmittedFormatting::TIME_LIMIT)
            .format_or_keep_unformatted(FakeStages::emitted(FakePrograms::PRINTS_A_SUM));
        assert!(written.is_ok_and(|written| written.formatting.is_some()));
    }

    #[test]
    fn keeps_the_rust_unformatted_when_formatting_fails() {
        let fixture = FormattingFixture::new();
        let written = fixture
            .formatting(Duration::ZERO)
            .format_or_keep_unformatted(FakeStages::emitted(FakePrograms::PRINTS_A_SUM));
        assert!(written.is_ok_and(|written| written.formatting.is_none()));
    }
}

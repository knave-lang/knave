use std::env::{self, VarError};
use std::time::Duration;

use super::exit_status::ExitStatus;
use super::standard_error::StandardError;
use crate::build::EmittedFormatting;

#[derive(Debug, PartialEq)]
pub struct RustfmtTimeLimit {
    limit: Duration,
}

#[derive(Debug, PartialEq)]
pub struct MalformedTimeLimit {
    variable: String,
}

impl RustfmtTimeLimit {
    pub fn from_environment() -> Result<Self, MalformedTimeLimit> {
        let variable = EmittedFormatting::time_limit_variable();
        let value = env::var(&variable);
        Self::parsed(variable, value)
    }

    pub fn formatting(&self) -> EmittedFormatting {
        EmittedFormatting::within(self.limit)
    }

    fn parsed(
        variable: String,
        value: Result<String, VarError>,
    ) -> Result<Self, MalformedTimeLimit> {
        match value {
            Err(VarError::NotPresent) => Ok(Self::within(EmittedFormatting::TIME_LIMIT)),
            value => value
                .ok()
                .and_then(|milliseconds| milliseconds.parse().ok())
                .map(|milliseconds| Self::within(Duration::from_millis(milliseconds)))
                .ok_or(MalformedTimeLimit { variable }),
        }
    }

    fn within(limit: Duration) -> Self {
        Self { limit }
    }
}

impl MalformedTimeLimit {
    pub fn report(&self) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: `{}` must be a whole number of milliseconds",
            self.variable
        ));
        ExitStatus::UsageError
    }
}

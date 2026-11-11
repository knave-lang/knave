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

#[cfg(test)]
mod tests {
    use std::env::VarError;
    use std::time::Duration;

    use super::{MalformedTimeLimit, RustfmtTimeLimit};
    use crate::build::EmittedFormatting;
    use crate::cli::exit_status::ExitStatus;

    struct TimeLimitFixture;

    impl TimeLimitFixture {
        const VARIABLE: &str = "FAKE_RUSTFMT_TIME_LIMIT";

        fn parsed(value: Result<&str, VarError>) -> Result<RustfmtTimeLimit, MalformedTimeLimit> {
            RustfmtTimeLimit::parsed(Self::VARIABLE.to_string(), value.map(str::to_string))
        }

        fn malformed() -> MalformedTimeLimit {
            MalformedTimeLimit {
                variable: Self::VARIABLE.to_string(),
            }
        }
    }

    #[test]
    fn falls_back_to_the_default_limit_without_the_variable() {
        let limit = TimeLimitFixture::parsed(Err(VarError::NotPresent));
        assert_eq!(
            limit,
            Ok(RustfmtTimeLimit::within(EmittedFormatting::TIME_LIMIT))
        );
    }

    #[test]
    fn reads_a_whole_number_of_milliseconds() {
        let limit = TimeLimitFixture::parsed(Ok("250"));
        assert_eq!(
            limit,
            Ok(RustfmtTimeLimit::within(Duration::from_millis(250)))
        );
    }

    #[test]
    fn refuses_a_limit_that_is_not_a_whole_number() {
        let limit = TimeLimitFixture::parsed(Ok("soon"));
        assert_eq!(limit, Err(TimeLimitFixture::malformed()));
    }

    #[test]
    fn reports_a_malformed_limit_as_a_usage_error() {
        assert_eq!(
            TimeLimitFixture::malformed().report(),
            ExitStatus::UsageError
        );
    }
}

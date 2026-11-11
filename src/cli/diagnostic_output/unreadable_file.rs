use std::fmt::{self, Display};
use std::io;
use std::path::Path;

use crate::cli::exit_status::ExitStatus;
use crate::cli::standard_error::StandardError;

pub struct UnreadableFile<'a> {
    path: &'a Path,
    error: &'a io::Error,
}

impl<'a> UnreadableFile<'a> {
    pub const EXIT_STATUS: ExitStatus = ExitStatus::UsageError;

    pub fn new(path: &'a Path, error: &'a io::Error) -> Self {
        Self { path, error }
    }

    pub fn report(&self) -> ExitStatus {
        StandardError::write_line(format_args!("error: {self}"));
        Self::EXIT_STATUS
    }
}

impl Display for UnreadableFile<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cannot read `{}`: {}",
            self.path.display(),
            self.error
        )
    }
}

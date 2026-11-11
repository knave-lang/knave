use std::fmt::{self, Display};
use std::io;
use std::path::Path;

use super::MessageOutput;
use crate::cli::exit_status::ExitStatus;

pub struct UnreadableFile<'a> {
    path: &'a Path,
    error: &'a io::Error,
}

impl<'a> UnreadableFile<'a> {
    const EXIT_STATUS: ExitStatus = ExitStatus::UsageError;

    pub fn new(path: &'a Path, error: &'a io::Error) -> Self {
        Self { path, error }
    }

    pub fn report_to(&self, output: &dyn MessageOutput) -> ExitStatus {
        output.write_error(&self.to_string());
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

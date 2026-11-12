use std::io;
use std::path::{Path, PathBuf};

use super::exit_status::ExitStatus;
use super::standard_error::StandardError;

pub struct WorkingDirectory;

impl WorkingDirectory {
    const PATH: &str = ".";

    pub fn path() -> &'static Path {
        Path::new(Self::PATH)
    }

    pub fn resolved() -> Result<PathBuf, ExitStatus> {
        Self::resolve(Self::path())
    }

    pub fn resolve(directory: &Path) -> Result<PathBuf, ExitStatus> {
        Self::path()
            .join(directory)
            .canonicalize()
            .map_err(|error| Self::report_unresolvable(directory, &error))
    }

    fn report_unresolvable(directory: &Path, error: &io::Error) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: cannot resolve `{}`: {error}",
            directory.display()
        ));
        ExitStatus::UsageError
    }
}

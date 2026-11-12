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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::WorkingDirectory;
    use crate::cli::exit_status::ExitStatus;
    use crate::cli::temporary_directory::TemporaryDirectory;

    #[test]
    fn resolves_a_directory_to_its_absolute_path() {
        let directory = TemporaryDirectory::create().expect("a temporary directory");
        let resolved = WorkingDirectory::resolve(directory.path());
        assert!(resolved.is_ok_and(|resolved| resolved.is_absolute()));
    }

    #[test]
    fn rejects_a_missing_directory_as_a_usage_error() {
        let directory = TemporaryDirectory::create().expect("a temporary directory");
        let missing = directory.path().join("missing");
        assert_eq!(
            WorkingDirectory::resolve(Path::new(&missing)),
            Err(ExitStatus::UsageError)
        );
    }
}

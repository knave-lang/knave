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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io;
    use std::path::Path;

    use super::UnreadableFile;
    use crate::cli::diagnostic_output::MessageOutput;
    use crate::cli::exit_status::ExitStatus;

    #[derive(Default)]
    struct RecordedErrors {
        errors: RefCell<Vec<String>>,
    }

    impl MessageOutput for RecordedErrors {
        fn write_error(&self, message: &str) {
            self.errors.borrow_mut().push(message.to_string());
        }
    }

    struct UnreadableFixture {
        error: io::Error,
        output: RecordedErrors,
    }

    impl UnreadableFixture {
        const PATH: &str = "notes/missing.knv";
        const CAUSE: &str = "no such file";
        const DESCRIPTION: &str = "cannot read `notes/missing.knv`: no such file";

        fn new() -> Self {
            Self {
                error: io::Error::new(io::ErrorKind::NotFound, Self::CAUSE),
                output: RecordedErrors::default(),
            }
        }

        fn file(&self) -> UnreadableFile<'_> {
            UnreadableFile::new(Path::new(Self::PATH), &self.error)
        }

        fn report(&self) -> ExitStatus {
            self.file().report_to(&self.output)
        }

        fn errors(&self) -> Vec<String> {
            self.output.errors.borrow().clone()
        }
    }

    #[test]
    fn describes_the_path_and_the_cause() {
        let fixture = UnreadableFixture::new();
        assert_eq!(fixture.file().to_string(), UnreadableFixture::DESCRIPTION);
    }

    #[test]
    fn writes_its_description_once_as_an_error() {
        let fixture = UnreadableFixture::new();
        fixture.report();
        assert_eq!(fixture.errors(), [UnreadableFixture::DESCRIPTION]);
    }

    #[test]
    fn reports_a_usage_error() {
        let fixture = UnreadableFixture::new();
        assert_eq!(fixture.report(), ExitStatus::UsageError);
    }
}

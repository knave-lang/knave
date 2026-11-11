use std::fmt::Display;
use std::io::{self, ErrorKind, StdoutLock, Write};

use super::exit_status::ExitStatus;
use super::standard_error::StandardError;

pub struct StandardOutput<W: Write> {
    writer: W,
}

impl StandardOutput<StdoutLock<'static>> {
    pub fn locked() -> Self {
        Self::new(io::stdout().lock())
    }
}

impl<W: Write> StandardOutput<W> {
    fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn write(mut self, answer: impl Display) -> ExitStatus {
        match self.write_and_flush(answer) {
            Err(error) if error.kind() != ErrorKind::BrokenPipe => Self::report_unwritable(&error),
            _ => ExitStatus::Success,
        }
    }

    fn write_and_flush(&mut self, answer: impl Display) -> io::Result<()> {
        write!(self.writer, "{answer}")?;
        self.writer.flush()
    }

    fn report_unwritable(error: &io::Error) -> ExitStatus {
        StandardError::write_line(format_args!("error: cannot write standard output: {error}"));
        ExitStatus::UsageError
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, ErrorKind, Write};

    use super::StandardOutput;
    use crate::cli::exit_status::ExitStatus;

    struct FailingWriter {
        kind: ErrorKind,
    }

    impl Write for FailingWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(self.kind))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct UnflushableWriter;

    impl Write for UnflushableWriter {
        fn write(&mut self, written: &[u8]) -> io::Result<usize> {
            Ok(written.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::from(ErrorKind::StorageFull))
        }
    }

    struct OutputFixture;

    impl OutputFixture {
        const ANSWER: &str = "(fake answer)";

        fn written() -> (ExitStatus, String) {
            let mut written = Vec::new();
            let status = StandardOutput::new(&mut written).write(Self::ANSWER);
            let text = String::from_utf8(written).expect("the answer is text");
            (status, text)
        }

        fn failing_with(kind: ErrorKind) -> ExitStatus {
            StandardOutput::new(FailingWriter { kind }).write(Self::ANSWER)
        }

        fn unflushable() -> ExitStatus {
            StandardOutput::new(UnflushableWriter).write(Self::ANSWER)
        }
    }

    #[test]
    fn writes_the_answer_and_succeeds() {
        let (status, text) = OutputFixture::written();
        assert_eq!(status, ExitStatus::Success);
        assert_eq!(text, OutputFixture::ANSWER);
    }

    #[test]
    fn treats_a_closed_reader_as_success() {
        let status = OutputFixture::failing_with(ErrorKind::BrokenPipe);
        assert_eq!(status, ExitStatus::Success);
    }

    #[test]
    fn reports_any_other_failed_write_as_a_usage_error() {
        let status = OutputFixture::failing_with(ErrorKind::StorageFull);
        assert_eq!(status, ExitStatus::UsageError);
    }

    #[test]
    fn reports_a_failed_flush_as_a_usage_error() {
        assert_eq!(OutputFixture::unflushable(), ExitStatus::UsageError);
    }
}

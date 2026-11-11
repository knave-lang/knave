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

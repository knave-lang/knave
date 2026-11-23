use super::exit_status::ExitStatus;
use super::standard_error::StandardError;

pub enum TaskAddress<'w> {
    Bare(&'w str),
    Qualified { library: &'w str, task: &'w str },
}

impl<'w> TaskAddress<'w> {
    const SEPARATOR: char = '/';

    pub fn parse(written: &'w str) -> Result<Self, ExitStatus> {
        let Some((library, task)) = written.split_once(Self::SEPARATOR) else {
            return Ok(Self::Bare(written));
        };
        if library.is_empty() || task.is_empty() || task.contains(Self::SEPARATOR) {
            return Err(Self::report_malformed(written));
        }
        Ok(Self::Qualified { library, task })
    }

    fn report_malformed(written: &str) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: `{written}` is not a task name\n\
             help: name a task as `<task>` or `<library>{}<task>`",
            Self::SEPARATOR
        ));
        ExitStatus::UsageError
    }
}

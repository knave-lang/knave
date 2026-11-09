use std::io;
use std::path::Path;
use std::process::{self, Command};

use super::exit_status::ExitStatus;
use super::program_file::ProgramFile;
use super::program_signal::ProgramSignal;
use super::standard_error::StandardError;

pub enum FinishedProgram {
    Exited(process::ExitStatus),
    NotStarted,
}

impl FinishedProgram {
    pub fn run(binary: &Path) -> Self {
        Self::run_with(binary, &[])
    }

    pub fn run_with(binary: &Path, arguments: &[String]) -> Self {
        Self::of(binary, Command::new(binary).args(arguments).status())
    }

    pub fn run_in(binary: &Path, arguments: &[String], directory: &Path) -> Self {
        let binary = ProgramFile::canonical(binary).unwrap_or_else(|| binary.to_path_buf());
        let directory =
            ProgramFile::canonical(directory).unwrap_or_else(|| directory.to_path_buf());
        let status = Command::new(&binary)
            .args(arguments)
            .current_dir(directory)
            .status();
        Self::of(&binary, status)
    }

    pub fn of(binary: &Path, status: io::Result<process::ExitStatus>) -> Self {
        match status {
            Ok(status) => Self::Exited(status),
            Err(error) => {
                StandardError::write_line(format_args!(
                    "error: cannot run `{}`: {error}",
                    binary.display()
                ));
                Self::NotStarted
            }
        }
    }

    pub fn status(&self) -> ExitStatus {
        let Self::Exited(status) = self else {
            return ExitStatus::Environment;
        };
        match ProgramSignal::of(*status) {
            Some(signal) => ExitStatus::KilledBy(signal),
            None => ExitStatus::Program(Self::code(*status)),
        }
    }

    fn code(status: process::ExitStatus) -> u8 {
        status
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .unwrap_or(ProgramSignal::UNREPORTABLE_CODE)
    }
}

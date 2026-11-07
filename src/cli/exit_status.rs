use std::io;
use std::process::ExitCode;

use super::program_signal::ProgramSignal;
use super::standard_error::StandardError;
use crate::build::InterruptSignal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitStatus {
    Success,
    ProgramErrors,
    UsageError,
    Environment,
    Program(u8),
    Interrupted,
    KilledBy(ProgramSignal),
}

impl ExitStatus {
    pub const SIGNAL_BASE: u8 = 128;

    pub fn code(self) -> u8 {
        match self {
            ExitStatus::Success => 0,
            ExitStatus::ProgramErrors => 1,
            ExitStatus::UsageError | ExitStatus::Environment => 2,
            ExitStatus::Program(code) => code,
            ExitStatus::Interrupted => Self::SIGNAL_BASE + InterruptSignal::NUMBER,
            ExitStatus::KilledBy(signal) => signal.shell_code(),
        }
    }

    pub fn is_interrupted(self) -> bool {
        match self {
            ExitStatus::Interrupted => true,
            ExitStatus::KilledBy(signal) => signal.is_interrupt(),
            _ => false,
        }
    }

    pub fn die_by_the_program_signal(self) {
        if let ExitStatus::KilledBy(signal) = self {
            signal.kill_this_process();
        }
    }

    pub fn report_unreadable_input(error: &io::Error) -> Self {
        StandardError::write_line(format_args!("error: cannot read standard input: {error}"));
        Self::UsageError
    }
}

impl From<ExitStatus> for ExitCode {
    fn from(status: ExitStatus) -> Self {
        ExitCode::from(status.code())
    }
}

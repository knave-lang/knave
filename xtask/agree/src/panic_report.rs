use std::process::ExitStatus;

use crate::timed_command::Finished;
use crate::verdict::{ExitKind, RunOutcome};

pub struct PanicReport {
    exit: ExitKind,
    stderr: Vec<u8>,
}

impl PanicReport {
    const PANIC_CODE: i32 = 101;
    const THREAD_PREFIX: &str = "thread '";
    const PANICKED_AT: &str = " panicked at ";
    const OVERFLOWED_STACK: &str = "has overflowed its stack";
    const FATAL_RUNTIME_ERROR: &str = "fatal runtime error: ";
    const BACKTRACE_NOTE: &str =
        "note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace";

    pub fn outcome(finished: Finished) -> RunOutcome {
        let report = Self::of(finished.status, &finished.stderr);
        RunOutcome {
            stdout: finished.stdout,
            stderr: report.stderr,
            exit: report.exit,
        }
    }

    fn of(status: ExitStatus, stderr: &[u8]) -> Self {
        let text = String::from_utf8_lossy(stderr);
        let lines: Vec<&str> = text.lines().collect();
        let panic_line = lines.iter().position(|line| Self::is_panic_line(line));
        let overflowed = lines
            .iter()
            .any(|line| line.contains(Self::OVERFLOWED_STACK));
        let exit = match (status.code(), panic_line) {
            _ if overflowed => ExitKind::StackOverflow,
            (Some(Self::PANIC_CODE), Some(position)) => {
                ExitKind::Panic(Self::message(&lines[position + 1..]))
            }
            (Some(code), _) => ExitKind::Code(code),
            (None, _) => ExitKind::Signal(Self::signal(status)),
        };
        Self {
            exit,
            stderr: Self::program_lines(&lines, panic_line),
        }
    }

    fn is_panic_line(line: &str) -> bool {
        line.starts_with(Self::THREAD_PREFIX)
            && (line.contains(Self::PANICKED_AT) || line.contains(Self::OVERFLOWED_STACK))
    }

    fn message(after_panic_line: &[&str]) -> String {
        after_panic_line
            .iter()
            .take_while(|line| **line != Self::BACKTRACE_NOTE)
            .copied()
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn program_lines(lines: &[&str], panic_line: Option<usize>) -> Vec<u8> {
        let mut written = &lines[..panic_line.unwrap_or(lines.len())];
        if panic_line.is_some() && written.last() == Some(&"") {
            written = &written[..written.len() - 1];
        }
        written
            .iter()
            .filter(|line| !line.starts_with(Self::FATAL_RUNTIME_ERROR))
            .map(|line| format!("{line}\n"))
            .collect::<String>()
            .into_bytes()
    }

    #[cfg(unix)]
    fn signal(status: ExitStatus) -> i32 {
        use std::os::unix::process::ExitStatusExt;
        status.signal().unwrap_or_default()
    }

    #[cfg(not(unix))]
    fn signal(_status: ExitStatus) -> i32 {
        0
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;

    use super::PanicReport;
    use crate::timed_command::Finished;
    use crate::verdict::{ExitKind, RunOutcome};

    struct PanicFixture;

    impl PanicFixture {
        const PANICKED: &str = "reading\n\nthread 'main' (1234) panicked at src/main.rs:5:25:\nindex out of bounds\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n";
        const OVERFLOWED: &str =
            "\nthread 'main' has overflowed its stack\nfatal runtime error: stack overflow\n";
        const ABORT: i32 = 6;

        fn exited(code: i32) -> ExitStatus {
            ExitStatus::from_raw(code << 8)
        }
    }

    #[test]
    fn keeps_the_panic_message_and_only_what_the_program_wrote() {
        let outcome = PanicReport::outcome(Finished {
            status: PanicFixture::exited(101),
            stdout: b"1\n".to_vec(),
            stderr: PanicFixture::PANICKED.as_bytes().to_vec(),
        });
        assert_eq!(
            outcome,
            RunOutcome {
                stdout: b"1\n".to_vec(),
                stderr: b"reading\n".to_vec(),
                exit: ExitKind::Panic("index out of bounds".to_string()),
            }
        );
    }

    #[test]
    fn tells_a_stack_overflow_from_a_signal() {
        let aborted = ExitStatus::from_raw(PanicFixture::ABORT);
        let overflow = PanicReport::of(aborted, PanicFixture::OVERFLOWED.as_bytes());
        assert_eq!(overflow.exit, ExitKind::StackOverflow);
        assert!(overflow.stderr.is_empty());
        assert_eq!(
            PanicReport::of(aborted, b"").exit,
            ExitKind::Signal(PanicFixture::ABORT)
        );
    }

    #[test]
    fn passes_an_exit_code_and_the_whole_standard_error() {
        let report = PanicReport::of(PanicFixture::exited(3), b"bad input\n");
        assert_eq!(report.exit, ExitKind::Code(3));
        assert_eq!(report.stderr, b"bad input\n");
    }
}

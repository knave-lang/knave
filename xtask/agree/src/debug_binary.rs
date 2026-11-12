use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::panic_report::PanicReport;
use crate::subject_program::SubjectProgram;
use crate::timed_command::{Completion, TimedCommand};
use crate::verdict::{ExitKind, RunOutcome, Verdict};

pub struct DebugBinary {
    path: PathBuf,
}

impl DebugBinary {
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn run(&self, program: &SubjectProgram, directory: &Path, limit: Duration) -> Verdict {
        let mut command = Command::new(&self.path);
        command.args(&program.arguments).current_dir(directory);
        match TimedCommand::new(command)
            .with_stdin(program.stdin.clone())
            .run(limit)
        {
            Ok(Completion::Finished(finished)) => Verdict::Ran(PanicReport::outcome(finished)),
            Ok(Completion::TimedOut) => Verdict::Ran(RunOutcome {
                stdout: Vec::new(),
                stderr: Vec::new(),
                exit: ExitKind::TimedOut,
            }),
            Err(error) => Verdict::Unsupported(format!("cannot run the program: {error}")),
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::env;
    use std::path::PathBuf;
    use std::time::Duration;

    use super::DebugBinary;
    use crate::subject_program::SubjectProgram;
    use crate::verdict::{ExitKind, Verdict};

    struct BinaryFixture;

    impl BinaryFixture {
        const SHORT_LIMIT: Duration = Duration::from_millis(100);

        fn sleeping_program() -> SubjectProgram {
            let mut program = SubjectProgram::of_entry(
                "sleeper".to_string(),
                PathBuf::from("main.knv"),
                String::new(),
            );
            program.arguments = vec!["30".to_string()];
            program
        }
    }

    #[test]
    fn a_program_past_its_limit_ran_and_timed_out() {
        let verdict = DebugBinary::at(PathBuf::from("sleep")).run(
            &BinaryFixture::sleeping_program(),
            &env::temp_dir(),
            BinaryFixture::SHORT_LIMIT,
        );
        assert!(matches!(verdict, Verdict::Ran(outcome) if outcome.exit == ExitKind::TimedOut));
    }

    #[test]
    fn a_binary_that_cannot_start_is_unsupported() {
        let verdict = DebugBinary::at(PathBuf::from("/nonexistent/program")).run(
            &BinaryFixture::sleeping_program(),
            &env::temp_dir(),
            BinaryFixture::SHORT_LIMIT,
        );
        assert!(matches!(verdict, Verdict::Unsupported(_)));
    }
}

use std::io;
use std::path::PathBuf;

use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::standard_error::StandardError;
use super::terminal::Terminal;
use super::test_binary::TestBinary;
use super::test_entries::TestEntries;
use super::test_report::{TestOutcome, TestReport};
use super::test_run::{TestRun, TestRunError};
use super::test_runner_program::TestRunnerProgram;
use super::value_marker::ValueMarker;
use crate::build::BuildProfile;
use crate::core::ModulePath;
use crate::emit::EmittedCrate;

pub struct TestCommand {
    entries: TestEntries,
    selection: TestSelection,
    runner: TestRunnerProgram,
    profile: BuildProfile,
}

pub enum TestSelection {
    Every,
    Containing(String),
    Exactly(String),
}

impl TestCommand {
    pub fn new(
        path: Option<PathBuf>,
        selection: TestSelection,
        untested: Option<String>,
        profile: BuildProfile,
    ) -> Self {
        Self {
            entries: TestEntries::new(path),
            selection,
            runner: TestRunnerProgram::new(untested.map(ModulePath::new)),
            profile,
        }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        match self.test(terminal) {
            Ok(status) | Err(status) => status,
        }
    }

    fn test(&self, terminal: &mut Terminal) -> Result<ExitStatus, ExitStatus> {
        let loaded = self.entries.load(terminal)?;
        let program = self.runner.program(terminal, loaded.module)?;
        let checked = terminal.checking().check(&program)?;
        let emitted = terminal.emission()?.emit(&checked)?;
        let marker = ValueMarker::random();
        let outcome = self.build_and_run(terminal, &emitted, loaded.project.as_ref(), &marker)?;
        self.note_no_selected_test(terminal, &outcome);
        Ok(Self::status(&outcome))
    }

    fn build_and_run(
        &self,
        terminal: &mut Terminal,
        emitted: &EmittedCrate,
        project: Option<&LocatedProject>,
        marker: &ValueMarker,
    ) -> Result<TestOutcome, ExitStatus> {
        let binary = TestBinary::build(terminal, emitted, project, self.profile)?;
        let mut report = TestReport::new(io::stdout().lock());
        TestRun::new(binary.path(), self.selection.arguments())
            .run(marker, &mut report)
            .map_err(Self::report_run_error)
    }

    fn note_no_selected_test(&self, terminal: &Terminal, outcome: &TestOutcome) {
        if outcome.selected > 0 {
            return;
        }
        if let Some(note) = self.selection.unmatched_note() {
            terminal.source_report().note(&note);
        }
    }

    fn status(outcome: &TestOutcome) -> ExitStatus {
        if outcome.all_passed {
            ExitStatus::Success
        } else {
            ExitStatus::ProgramErrors
        }
    }

    fn report_run_error(error: TestRunError) -> ExitStatus {
        match error {
            TestRunError::NotStarted => ExitStatus::Environment,
            TestRunError::Unwritable(error) if error.kind() == io::ErrorKind::BrokenPipe => {
                ExitStatus::Success
            }
            TestRunError::Unwritable(error) => {
                StandardError::write_line(format_args!(
                    "error: cannot write standard output: {error}"
                ));
                ExitStatus::UsageError
            }
        }
    }
}

impl TestSelection {
    const EXACT_FLAG: &'static str = "--exact";
    const FILTER_FLAG: &'static str = "--filter";

    pub fn of(filter: Option<String>, exact: Option<String>) -> Self {
        match (filter, exact) {
            (_, Some(name)) => Self::Exactly(name),
            (Some(filter), None) => Self::Containing(filter),
            (None, None) => Self::Every,
        }
    }

    pub fn report_positional_filter(filter: &str) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: unexpected argument `{filter}` after the path\n\
             help: to run the tests whose full name contains it, write `{} {filter}`",
            Self::FILTER_FLAG
        ));
        ExitStatus::UsageError
    }

    fn arguments(&self) -> Vec<String> {
        match self {
            Self::Every => Vec::new(),
            Self::Containing(filter) => vec![filter.clone()],
            Self::Exactly(name) => vec![Self::EXACT_FLAG.to_string(), name.clone()],
        }
    }

    fn unmatched_note(&self) -> Option<String> {
        match self {
            Self::Every => None,
            Self::Containing(filter) => Some(format!("no test name contains `{filter}`")),
            Self::Exactly(name) => Some(format!("no test is named `{name}`")),
        }
    }
}

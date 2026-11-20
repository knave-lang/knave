use super::evaluation_request::EvaluationRequest;
use super::exit_status::ExitStatus;
use super::module_of_sources::ModuleOfSources;
use super::standard_error::StandardError;
use super::terminal::Terminal;
use crate::core::ModulePath;
use crate::diagnostic::Diagnostics;
use crate::expand::{ExpansionSession, Program};

pub struct TestRunnerProgram {
    untested: Option<ModulePath>,
}

impl TestRunnerProgram {
    const RUNNER_SOURCE: &'static str = "<test runner>";

    pub fn new(untested: Option<ModulePath>) -> Self {
        Self { untested }
    }

    pub fn program(
        &self,
        terminal: &mut Terminal,
        module: ModuleOfSources,
    ) -> Result<Program, ExitStatus> {
        let request = EvaluationRequest {
            module,
            evaluated: Vec::new(),
        };
        let runner = self.runner_text();
        let program = terminal
            .expansion()
            .sessions()
            .within_evaluation(&request, |session| self.entered_runner(session, &runner))?;
        program.ok_or_else(|| self.report_unloaded_module())
    }

    fn entered_runner(
        &self,
        mut session: ExpansionSession<'_>,
        runner: &str,
    ) -> Result<Option<Program>, Diagnostics> {
        if let Some(module) = &self.untested
            && !session.loads(module)
        {
            return Ok(None);
        }
        let source = session.add_source(Self::RUNNER_SOURCE, runner);
        let entry = session.test_runner_entry(source)?;
        let mut program = session.finish()?;
        program.enter(&entry);
        Ok(Some(program))
    }

    fn runner_text(&self) -> String {
        let untested = self
            .untested
            .as_ref()
            .map(|module| format!(" :untested {module}"))
            .unwrap_or_default();
        format!("(run-tests{untested})")
    }

    fn report_unloaded_module(&self) -> ExitStatus {
        let module = self
            .untested
            .as_ref()
            .expect("only `--untested` names a module");
        StandardError::write_line(format_args!(
            "error: the program does not load module `{module}`"
        ));
        ExitStatus::UsageError
    }
}

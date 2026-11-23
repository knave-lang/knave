use super::exit_status::ExitStatus;
use super::finished_program::FinishedProgram;
use super::task_catalog::TaskCatalog;
use super::task_runners::TaskRunners;
use super::task_target::TaskTarget;
use super::terminal::Terminal;
use crate::build::BuildProfile;

#[derive(Clone, Copy)]
pub struct TaskInvocation<'a> {
    pub arguments: &'a [String],
    pub profile: BuildProfile,
}

impl TaskInvocation<'_> {
    pub fn run(
        &self,
        terminal: &mut Terminal,
        catalog: &TaskCatalog,
        target: &dyn TaskTarget,
    ) -> Result<ExitStatus, ExitStatus> {
        let binary = TaskRunners::new(catalog).build(terminal, target, self.profile)?;
        Ok(FinishedProgram::run_in(&binary, self.arguments, target.root()).status())
    }
}

use std::path::PathBuf;

use super::exit_status::ExitStatus;
use super::project_task::{ProjectTask, TaskName};
use super::task_catalog::TaskCatalog;
use super::terminal::Terminal;
use crate::build::BuildProfile;
use crate::reader::StringLiteral;

pub struct TaskRunners<'c> {
    catalog: &'c TaskCatalog,
}

impl<'c> TaskRunners<'c> {
    pub fn new(catalog: &'c TaskCatalog) -> Self {
        Self { catalog }
    }

    pub fn build(
        &self,
        terminal: &mut Terminal,
        task: &ProjectTask<'_>,
        profile: BuildProfile,
    ) -> Result<PathBuf, ExitStatus> {
        let signatures = self
            .catalog
            .signatures(terminal, std::slice::from_ref(task.name()))?;
        let runner = Self::runner(task.name(), &signatures[0]);
        let entered = self.catalog.entered_program(terminal, &runner)?;
        let checked = terminal.check_expanded(&entered.program)?;
        let emitted = terminal.emit_checked(&checked)?;
        terminal.build_task(&emitted, task, profile)
    }

    pub fn check(&self, terminal: &mut Terminal) -> Result<(), ExitStatus> {
        let names = self.catalog.names(terminal)?;
        let signatures = self.catalog.signatures(terminal, &names)?;
        let runners: Vec<String> = names
            .iter()
            .zip(&signatures)
            .map(|(name, signature)| Self::runner(name, signature))
            .collect();
        let (block, _) = TaskCatalog::block_of(&runners);
        let entered = self.catalog.entered_program(terminal, &block)?;
        terminal.check_expanded(&entered.program).map(|_| ())
    }

    fn runner(name: &TaskName, signature: &str) -> String {
        format!(
            "(run-task {} {})",
            StringLiteral::quoted(name.as_str()),
            StringLiteral::quoted(signature)
        )
    }
}

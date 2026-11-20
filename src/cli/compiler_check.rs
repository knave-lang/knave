use std::path::Path;

use super::exit_status::ExitStatus;
use super::located_project::ProjectRoot;
use super::terminal::Terminal;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::language::Language;
use crate::project::{CompilerVersion, ReleaseVersion};

pub struct CompilerCheck<'t> {
    terminal: &'t Terminal,
}

impl CompilerCheck<'_> {
    pub fn new(terminal: &Terminal) -> CompilerCheck<'_> {
        CompilerCheck { terminal }
    }

    pub fn refuse_before_loading(&self, root: &ProjectRoot) -> Result<(), ExitStatus> {
        let project_file = root.shown()?.join(Language::project_file_name());
        match self.printed(&project_file, CompilerVersion::refusal) {
            Some(refused) => Err(refused),
            None => Ok(()),
        }
    }

    pub fn report(&self, root: &ProjectRoot) -> Result<(), ExitStatus> {
        self.warn(&root.project_file())
    }

    pub fn report_entered(&self, root: &ProjectRoot) -> Result<(), ExitStatus> {
        self.warn(&root.shown()?.join(Language::project_file_name()))
    }

    fn warn(&self, project_file: &Path) -> Result<(), ExitStatus> {
        match self.printed(project_file, CompilerVersion::warning) {
            Some(ExitStatus::Success) | None => Ok(()),
            Some(failure) => Err(failure),
        }
    }

    fn printed(
        &self,
        project_file: &Path,
        problem: impl Fn(&CompilerVersion, ReleaseVersion) -> Option<Diagnostic>,
    ) -> Option<ExitStatus> {
        let mut scratch = self.terminal.scratch();
        let recorded = scratch
            .sources()
            .projects()
            .recorded_version(project_file)?;
        let found = problem(&recorded, ReleaseVersion::running())?;
        Some(
            scratch
                .source_report()
                .print(&Diagnostics::from_iter([found])),
        )
    }
}

use std::path::Path;

use super::exit_status::ExitStatus;
use super::located_project::ProjectRoot;
use super::terminal::Terminal;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::language::Language;
use crate::project::{Project, ReleaseVersion};

pub struct CompilerCheck<'t> {
    terminal: &'t Terminal,
}

impl CompilerCheck<'_> {
    pub fn new(terminal: &Terminal) -> CompilerCheck<'_> {
        CompilerCheck { terminal }
    }

    pub fn refuse_before_loading(&self, root: &ProjectRoot) -> Result<(), ExitStatus> {
        let mut scratch = self.terminal.scratch();
        let project_file = root.shown()?.join(Language::project_file_name());
        let Some(recorded) = scratch.sources().projects().recorded_version(&project_file) else {
            return Ok(());
        };
        let Some(refusal) = recorded.refusal(ReleaseVersion::running()) else {
            return Ok(());
        };
        Err(scratch
            .source_report()
            .print(&Diagnostics::from_iter([refusal])))
    }

    pub fn report(&self, root: &ProjectRoot) -> Result<(), ExitStatus> {
        let project_file = root.project_file();
        let mut scratch = self.terminal.scratch();
        let Some(project) = scratch.sources().projects().read_quietly(&project_file) else {
            return Ok(());
        };
        self.report_located(&project, &project_file)
    }

    pub fn report_located(&self, project: &Project, project_file: &Path) -> Result<(), ExitStatus> {
        if Self::problem_in(project).is_none() {
            return Ok(());
        }
        let mut scratch = self.terminal.scratch();
        let located_project = scratch.sources().projects().load(project_file)?;
        let Some(problem) = Self::problem_in(&located_project) else {
            return Ok(());
        };
        let mut problems = Diagnostics::default();
        problems.push(problem);
        match scratch.source_report().print(&problems) {
            ExitStatus::Success => Ok(()),
            failure => Err(failure),
        }
    }

    fn problem_in(project: &Project) -> Option<Diagnostic> {
        let recorded = project.compiler_version.as_ref()?;
        recorded.warning(ReleaseVersion::running())
    }
}

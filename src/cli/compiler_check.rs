use std::path::Path;

use super::exit_status::ExitStatus;
use super::located_project::ProjectRoot;
use super::terminal::Terminal;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::language::Language;
use crate::project::{Project, ReleaseVersion};

pub struct CompilerCheck;

impl CompilerCheck {
    pub fn refuse_before_loading(directory: &Path, terminal: &Terminal) -> Result<(), ExitStatus> {
        let Some(root) = ProjectRoot::shown_of(directory)? else {
            return Ok(());
        };
        let mut scratch = terminal.scratch();
        let project_file = root.join(Language::project_file_name());
        let Some(refusal) = scratch.sources().projects().version_refusal(&project_file) else {
            return Ok(());
        };
        Err(scratch
            .source_report()
            .print(&Diagnostics::from_iter([refusal])))
    }

    pub fn report(project_file: &Path, terminal: &Terminal) -> Result<(), ExitStatus> {
        let mut scratch = terminal.scratch();
        let Some(project) = scratch.sources().projects().read_quietly(project_file) else {
            return Ok(());
        };
        Self::report_located(&project, project_file, terminal)
    }

    pub fn report_located(
        project: &Project,
        project_file: &Path,
        terminal: &Terminal,
    ) -> Result<(), ExitStatus> {
        if Self::problem_in(project).is_none() {
            return Ok(());
        }
        let mut scratch = terminal.scratch();
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

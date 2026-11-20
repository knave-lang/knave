use std::path::Path;

use super::exit_status::ExitStatus;
use super::terminal::Terminal;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::language::Language;
use crate::project::{CompilerVersion, Project, ReleaseVersion};

type VersionProblem = fn(&CompilerVersion, ReleaseVersion) -> Option<Diagnostic>;

pub struct CompilerCheck {
    problem_of: VersionProblem,
}

impl CompilerCheck {
    pub const EVERY_MISMATCH: Self = Self {
        problem_of: CompilerVersion::problem,
    };
    pub const OLDER_COMPILER: Self = Self {
        problem_of: CompilerVersion::refusal,
    };

    pub fn refuse_before_loading(root: &Path, terminal: &Terminal) -> Result<(), ExitStatus> {
        let mut scratch = terminal.scratch();
        let project_file = root.join(Language::project_file_name());
        let Some(refusal) = scratch.sources().projects().version_refusal(&project_file) else {
            return Ok(());
        };
        Err(scratch
            .source_report()
            .print(&Diagnostics::from_iter([refusal])))
    }

    pub fn report(&self, project_file: &Path, terminal: &Terminal) -> Result<(), ExitStatus> {
        let mut scratch = terminal.scratch();
        let project = scratch.sources().projects().load(project_file)?;
        self.report_located(&project, project_file, terminal)
    }

    pub fn report_located(
        &self,
        project: &Project,
        project_file: &Path,
        terminal: &Terminal,
    ) -> Result<(), ExitStatus> {
        if self.problem_in(project).is_none() {
            return Ok(());
        }
        let mut scratch = terminal.scratch();
        let located_project = scratch.sources().projects().load(project_file)?;
        let Some(problem) = self.problem_in(&located_project) else {
            return Ok(());
        };
        let mut problems = Diagnostics::default();
        problems.push(problem);
        match scratch.source_report().print(&problems) {
            ExitStatus::Success => Ok(()),
            failure => Err(failure),
        }
    }

    fn problem_in(&self, project: &Project) -> Option<Diagnostic> {
        let recorded = project.compiler_version.as_ref()?;
        (self.problem_of)(recorded, ReleaseVersion::running())
    }
}

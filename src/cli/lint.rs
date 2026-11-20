use std::fs;
use std::path::Path;

use super::diagnostic_output::StandardErrorMessages;
use super::exit_status::ExitStatus;
use super::library_walk::FetchingStore;
use super::lint_path::LintPath;
use super::linted_programs::LintedPrograms;
use super::located_program::LocatedProgram;
use super::located_project::LocatedProject;
use super::path_scope::PathScope;
use super::project_programs::ProjectPrograms;
use super::standard_error::StandardError;
use super::task_catalog::TaskCatalog;
use super::terminal::Terminal;
use super::unreadable_file::UnreadableFile;

pub struct LintCommand {
    path: LintPath,
}

impl LintCommand {
    pub fn new(path: LintPath) -> Self {
        Self { path }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        ExitStatus::of(self.lint(terminal))
    }

    fn lint(self, terminal: &mut Terminal) -> Result<(), ExitStatus> {
        let output = TaskCatalog::user_output(terminal.diagnostic_output());
        terminal.configure(|settings| settings.rerouted_to(output));
        let (programs, scope) = self.programs(terminal)?;
        let loaded = programs.load(terminal);
        let linted = LintedPrograms::checked(terminal, loaded);
        let failure = linted.failure();
        let warnings = linted.warnings(terminal)?;
        let report = terminal.source_report();
        let shown = scope.kept(report.sources(), warnings);
        let printed = report.lints().print(&shown).passed();
        failure.map_or(printed, Err)
    }

    fn programs(
        &self,
        terminal: &mut Terminal,
    ) -> Result<(ProjectPrograms, PathScope), ExitStatus> {
        match self.path.path() {
            None => Self::whole_project(terminal),
            Some(path) => {
                Self::readable(path)?;
                if path.is_dir() {
                    Self::directory(terminal, path)
                } else {
                    Self::file(terminal, path)
                }
            }
        }
    }

    fn readable(path: &Path) -> Result<(), ExitStatus> {
        fs::metadata(path)
            .map(|_| ())
            .map_err(|error| UnreadableFile::new(path, &error).report_to(&StandardErrorMessages))
    }

    fn whole_project(terminal: &mut Terminal) -> Result<(ProjectPrograms, PathScope), ExitStatus> {
        let project = LocatedProgram::of_working_directory(terminal)?
            .into_project()
            .expect("a lint without a path lints a project");
        Ok((
            ProjectPrograms::of_project(project),
            PathScope::whole_project(),
        ))
    }

    fn directory(
        terminal: &mut Terminal,
        path: &Path,
    ) -> Result<(ProjectPrograms, PathScope), ExitStatus> {
        let project = LocatedProject::enter_from(path, terminal, FetchingStore::default())?
            .ok_or_else(|| Self::report_no_project(path))?;
        Ok((ProjectPrograms::of_project(project), PathScope::under(path)))
    }

    fn file(
        terminal: &mut Terminal,
        path: &Path,
    ) -> Result<(ProjectPrograms, PathScope), ExitStatus> {
        let located = LocatedProgram::of_file(path.to_path_buf(), terminal)?;
        let scope = match located.project() {
            Some(_) => PathScope::under(path),
            None => PathScope::whole_project(),
        };
        Ok((ProjectPrograms::of_file(located), scope))
    }

    fn report_no_project(directory: &Path) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: `{}` is in no project\n\
             help: name a file, or a directory inside a project",
            directory.display()
        ));
        ExitStatus::UsageError
    }
}

use std::rc::Rc;

use super::direct_library::DirectLibrary;
use super::entry_tree::EntryTree;
use super::errors_only_output::ErrorsOnlyOutput;
use super::exit_status::ExitStatus;
use super::lending_libraries::LendingLibraries;
use super::library_task::{LibraryTask, LibraryTaskSite};
use super::library_walk::FetchingStore;
use super::lint_levels::LintLevels;
use super::located_project::LocatedProject;
use super::task_catalog::TaskCatalog;
use super::task_invocation::TaskInvocation;
use super::task_name::TaskName;
use super::task_runners::TaskRunners;
use super::terminal::Terminal;
use super::terminal_settings::TerminalSettings;

pub struct LibraryTasks {
    tree: EntryTree,
    catalog: TaskCatalog,
}

impl LibraryTasks {
    pub fn within<T>(
        terminal: &mut Terminal,
        library: &DirectLibrary,
        work: impl FnOnce(&mut Terminal, &LibraryTasks) -> Result<T, ExitStatus>,
    ) -> Result<T, ExitStatus> {
        let saved = Self::quieten(terminal);
        let outcome = Self::load(terminal, library).and_then(|tasks| work(terminal, &tasks));
        terminal.configure(|_| saved);
        outcome
    }

    pub fn listing(&self, terminal: &mut Terminal, arguments: &str) -> Result<String, ExitStatus> {
        self.catalog.exports()?;
        self.catalog.listing(terminal, arguments)
    }

    pub fn check_lenders(
        terminal: &mut Terminal,
        lending: &LendingLibraries,
    ) -> Result<(), ExitStatus> {
        lending
            .lenders()
            .into_iter()
            .map(|library| Self::within(terminal, library, Self::check))
            .fold(Ok(()), Result::and)
    }

    fn check(terminal: &mut Terminal, tasks: &LibraryTasks) -> Result<(), ExitStatus> {
        TaskRunners::new(&tasks.catalog).check(terminal).map(|_| ())
    }

    pub fn run(
        &self,
        terminal: &mut Terminal,
        site: &LibraryTaskSite<'_>,
        invocation: TaskInvocation<'_>,
    ) -> Result<ExitStatus, ExitStatus> {
        self.catalog.exports()?;
        let library = self.project();
        let target = LibraryTask::new(site, library, TaskName::new(site.task.to_string()));
        library
            .crates()
            .copy_lock_into(target.crates())
            .map_err(|error| Self::report_lock(terminal, &error))?;
        invocation.run(terminal, &self.catalog, &target)
    }

    fn project(&self) -> &LocatedProject {
        self.tree
            .project()
            .expect("a library's task tree keeps its project")
    }

    fn quieten(terminal: &mut Terminal) -> TerminalSettings {
        let mut saved = None;
        terminal.configure(|settings| {
            let diagnostics = Rc::clone(settings.diagnostics());
            saved = Some(settings.rerouted_to(Rc::clone(&diagnostics)));
            settings.rerouted_to(Rc::new(ErrorsOnlyOutput::new(diagnostics)))
        });
        saved.expect("the settings were saved before they changed")
    }

    fn load(terminal: &mut Terminal, library: &DirectLibrary) -> Result<Self, ExitStatus> {
        let project =
            LocatedProject::enter_unchecked(library.site(), terminal, FetchingStore::default())?
                .expect("a listed library has a project file");
        terminal.configure(|settings| settings.leveling_lints_by(LintLevels::default()));
        let tree = EntryTree::of_tasks(project);
        let catalog = TaskCatalog::load(&tree, terminal, &library.arguments().export_check())?;
        catalog.names(terminal)?;
        Ok(Self { tree, catalog })
    }

    fn report_lock(terminal: &Terminal, error: &std::io::Error) -> ExitStatus {
        terminal
            .diagnostic_output()
            .write_error(&format!("cannot copy the library's `Cargo.lock`: {error}"));
        ExitStatus::Environment
    }
}

use std::rc::Rc;

use super::direct_library::DirectLibrary;
use super::entry_tree::EntryTree;
use super::errors_only_output::ErrorsOnlyOutput;
use super::exit_status::ExitStatus;
use super::export_status::ExportStatus;
use super::library_walk::FetchingStore;
use super::lint_levels::LintLevels;
use super::located_project::LocatedProject;
use super::task_catalog::TaskCatalog;
use super::task_runners::TaskRunners;
use super::terminal::Terminal;
use super::terminal_settings::TerminalSettings;

pub struct LibraryTasks {
    pub tree: EntryTree,
    pub catalog: TaskCatalog,
    pub exports: ExportStatus,
}

impl LibraryTasks {
    pub fn within<T>(
        terminal: &mut Terminal,
        library: &DirectLibrary,
        work: impl FnOnce(&mut Terminal, LibraryTasks) -> Result<T, ExitStatus>,
    ) -> Result<T, ExitStatus> {
        let saved = Self::quieten(terminal);
        let outcome = Self::load(terminal, library).and_then(|tasks| work(terminal, tasks));
        terminal.configure(|_| saved);
        outcome
    }

    pub fn check_lenders(
        terminal: &mut Terminal,
        libraries: &[DirectLibrary],
    ) -> Result<(), ExitStatus> {
        libraries
            .iter()
            .filter(|library| !library.exported_tasks.is_empty())
            .map(|library| Self::within(terminal, library, Self::check))
            .fold(Ok(()), Result::and)
    }

    fn check(terminal: &mut Terminal, tasks: LibraryTasks) -> Result<(), ExitStatus> {
        TaskRunners::new(&tasks.catalog)
            .check(terminal, tasks.exports)
            .map(|_| ())
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
            LocatedProject::enter_unchecked(&library.root, terminal, FetchingStore::default())?
                .expect("a listed library has a project file");
        terminal.configure(|settings| settings.leveling_lints_by(LintLevels::default()));
        let tree = EntryTree::of_tasks(project);
        let (catalog, exports) = TaskCatalog::load(&tree, terminal, Some(library))?;
        catalog.names(terminal)?;
        Ok(Self {
            tree,
            catalog,
            exports,
        })
    }
}

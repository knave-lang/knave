use std::path::{Path, PathBuf};

use rustc_hash::FxHashSet;

use super::entry_tree::EntryTree;
use super::exit_status::ExitStatus;
use super::library_tasks::LibraryTasks;
use super::loaded_programs::LoadedPrograms;
use super::located_program::LocatedProgram;
use super::located_project::LocatedProject;
use super::program_file::ProgramFile;
use super::project_program::ProjectProgram;
use super::source_files::SourceFiles;
use super::task_catalog::TaskCatalog;
use super::task_runners::TaskRunners;
use super::terminal::Terminal;
use super::test_entries::TestEntries;
use super::test_runner_program::TestRunnerProgram;

pub struct ProjectPrograms {
    scope: Scope,
}

enum Scope {
    File(Box<LocatedProgram>),
    Project(Box<LocatedProject>),
}

impl ProjectPrograms {
    pub fn of_file(located: LocatedProgram) -> Self {
        Self {
            scope: Scope::File(Box::new(located)),
        }
    }

    pub fn of_project(project: LocatedProject) -> Self {
        Self {
            scope: Scope::Project(Box::new(project)),
        }
    }

    pub fn load(self, terminal: &mut Terminal) -> LoadedPrograms {
        let mut loaded = LoadedPrograms::default();
        match self.scope {
            Scope::Project(project) => {
                Self::every(terminal, *project, &mut loaded);
            }
            Scope::File(located) => {
                let file = located.file().clone();
                match (*located).into_project() {
                    Some(project) => {
                        let tried = Self::every(terminal, project, &mut loaded);
                        if !Self::holds(&tried, &file) {
                            loaded.add(Self::entry(terminal, file));
                        }
                    }
                    None => loaded.add(Self::entry(terminal, file)),
                }
            }
        }
        loaded
    }

    fn every(
        terminal: &mut Terminal,
        project: LocatedProject,
        loaded: &mut LoadedPrograms,
    ) -> FxHashSet<PathBuf> {
        let entry_and_library_root = Self::load_entry_and_library_root(terminal, &project, loaded);
        let project = Self::tests(terminal, project, loaded);
        let project = Self::tasks(terminal, project, loaded);
        UnreachedWalk::new(terminal, &project, entry_and_library_root).load(terminal, loaded)
    }

    fn load_entry_and_library_root(
        terminal: &mut Terminal,
        project: &LocatedProject,
        loaded: &mut LoadedPrograms,
    ) -> FxHashSet<PathBuf> {
        let layout = project.entry_layout();
        let mut tried = FxHashSet::default();
        if !layout.skips_entry() {
            tried.extend(Self::load_canonical(terminal, layout.entry(), loaded));
        }
        let held = terminal.source_report().canonical_files_from(0);
        if let Some(library_root) = layout.library_root_outside(&held) {
            tried.extend(Self::load_canonical(terminal, library_root, loaded));
        }
        tried
    }

    fn load_canonical(
        terminal: &mut Terminal,
        file: ProgramFile,
        loaded: &mut LoadedPrograms,
    ) -> Option<PathBuf> {
        let canonical = ProgramFile::canonical(file.path());
        loaded.add(Self::entry(terminal, file));
        canonical
    }

    fn entry(terminal: &mut Terminal, file: ProgramFile) -> Result<ProjectProgram, ExitStatus> {
        let source = terminal.sources().load(file.path())?;
        let program = terminal.expansion().expand(&file, source)?;
        Ok(ProjectProgram { file, program })
    }

    fn tests(
        terminal: &mut Terminal,
        project: LocatedProject,
        loaded: &mut LoadedPrograms,
    ) -> LocatedProject {
        if !project.placement().tests_directory().is_dir() {
            return project;
        }
        let tree = TestEntries::project_tree(project);
        loaded.add(tree.load(terminal).and_then(|module| {
            let file = module.file.clone();
            let program = TestRunnerProgram::new(None).program(terminal, module)?;
            Ok(ProjectProgram { file, program })
        }));
        Self::project_of(tree)
    }

    fn tasks(
        terminal: &mut Terminal,
        project: LocatedProject,
        loaded: &mut LoadedPrograms,
    ) -> LocatedProject {
        let tree = EntryTree::of_tasks(project);
        if tree.holds_tasks() {
            match TaskCatalog::load(&tree, terminal, "") {
                Ok(catalog) => Self::task_program(terminal, &catalog, loaded),
                Err(status) => loaded.fail(status),
            }
        }
        let project = Self::project_of(tree);
        if let Err(status) = LibraryTasks::check_lenders(terminal, project.task_layout().lending())
        {
            loaded.fail(status);
        }
        project
    }

    fn task_program(terminal: &mut Terminal, catalog: &TaskCatalog, loaded: &mut LoadedPrograms) {
        if let Err(status) = catalog.exports() {
            loaded.fail(status);
        }
        let program = TaskRunners::new(catalog).program(terminal);
        loaded.add(program.map(|entered| ProjectProgram {
            file: entered.file,
            program: entered.program,
        }));
    }

    fn project_of(tree: EntryTree) -> LocatedProject {
        tree.into_project()
            .expect("a tree of a project keeps its project")
    }

    fn holds(tried: &FxHashSet<PathBuf>, file: &ProgramFile) -> bool {
        ProgramFile::canonical(file.path()).is_some_and(|named| tried.contains(&named))
    }
}

struct UnreachedWalk<'p> {
    project: &'p LocatedProject,
    tried: FxHashSet<PathBuf>,
}

impl<'p> UnreachedWalk<'p> {
    const CURRENT_FOLDER: &'static str = ".";

    fn new(
        terminal: &Terminal,
        project: &'p LocatedProject,
        first_tried: FxHashSet<PathBuf>,
    ) -> Self {
        let mut tried = terminal.source_report().canonical_files_from(0);
        tried.extend(first_tried);
        Self { project, tried }
    }

    fn load(mut self, terminal: &mut Terminal, loaded: &mut LoadedPrograms) -> FxHashSet<PathBuf> {
        let project = self.project;
        let paths = match SourceFiles::under(project.root().to_path_buf(), project).paths() {
            Ok(paths) => paths,
            Err(status) => {
                loaded.fail(status);
                return self.tried;
            }
        };
        for path in paths.into_iter().map(Self::without_current_folder) {
            if ProgramFile::canonical(&path).is_some_and(|canonical| self.tried.insert(canonical)) {
                let first_source = terminal.source_report().sources().next_id().index();
                let file = project.placement().file_within(path);
                loaded.add(ProjectPrograms::entry(terminal, file));
                self.tried
                    .extend(terminal.source_report().canonical_files_from(first_source));
            }
        }
        self.tried
    }

    fn without_current_folder(path: PathBuf) -> PathBuf {
        path.strip_prefix(Self::CURRENT_FOLDER)
            .map(Path::to_path_buf)
            .unwrap_or(path)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::rc::Rc;

    use super::ProjectPrograms;
    use crate::cli::library_walk::FetchingStore;
    use crate::cli::located_project::LocatedProject;
    use crate::cli::recorded_output::RecordedOutput;
    use crate::cli::temporary_directory::TemporaryDirectory;
    use crate::cli::terminal::Terminal;
    use crate::cli::terminal_settings::TerminalSettings;

    struct ChainFixture {
        directory: TemporaryDirectory,
    }

    impl ChainFixture {
        const FILES: [(&str, &str); 4] = [
            ("project.knv", "(defproject chain)\n"),
            ("src/main.knv", "(defn main () (println 1))\n"),
            (
                "src/lib/a.knv",
                "(use lib/b twice)\n\n(defn four () (twice 2))\n",
            ),
            (
                "src/lib/b.knv",
                "(export twice)\n\n(defn twice (x) (* 2 x))\n",
            ),
        ];
        const LIBRARY_FILES: [(&str, &str); 4] = [
            ("project.knv", "(defproject chain)\n"),
            (
                "src/lib.knv",
                "(use lib/a four)\n\n(export eight)\n\n(defn eight () (* 2 (four)))\n",
            ),
            (
                "src/lib/a.knv",
                "(use lib/b twice)\n\n(export four)\n\n(defn four () (twice 2))\n",
            ),
            (
                "src/lib/b.knv",
                "(export twice)\n\n(defn twice (x) (* 2 x))\n",
            ),
        ];

        fn new(files: &[(&str, &str)]) -> Self {
            let directory = TemporaryDirectory::create().expect("a scratch directory is created");
            for (path, text) in files {
                let file = directory.path().join(path);
                fs::create_dir_all(file.parent().expect("a file has a folder"))
                    .expect("the scratch directory is writable");
                fs::write(file, text).expect("the scratch directory is writable");
            }
            Self { directory }
        }

        fn loaded_programs(&self) -> usize {
            let mut terminal = Terminal::new(TerminalSettings::reporting_to(Rc::new(
                RecordedOutput::default(),
            )));
            let project =
                LocatedProject::enter_from(self.root(), &mut terminal, FetchingStore::default())
                    .expect("the project reads")
                    .expect("the folder is a project");
            let results = ProjectPrograms::of_project(project)
                .load(&mut terminal)
                .into_results();
            assert!(results.iter().all(Result::is_ok));
            results.len()
        }

        fn root(&self) -> &Path {
            self.directory.path()
        }
    }

    #[test]
    fn a_module_that_an_unreached_file_loads_is_not_loaded_again() {
        assert_eq!(ChainFixture::new(&ChainFixture::FILES).loaded_programs(), 2);
    }

    #[test]
    fn a_library_root_loads_before_the_modules_it_reaches() {
        let fixture = ChainFixture::new(&ChainFixture::LIBRARY_FILES);
        assert_eq!(fixture.loaded_programs(), 1);
    }
}

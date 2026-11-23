use std::path::{Path, PathBuf};

use super::diagnostic_output::StandardErrorMessages;
use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::module_names::ModuleNames;
use super::module_of_sources::ModuleOfSources;
use super::program_file::ProgramFile;
use super::skipped_directory::SkippedDirectory;
use super::taken_module_paths::TakenModulePaths;
use super::terminal::Terminal;
use super::tree_bounds::TreeBounds;
use super::tree_file::TreeFile;
use super::tree_modules::TreeModules;
use super::tree_walk::TreeWalk;
use super::unreadable_file::UnreadableFile;

pub struct EntryTree {
    path: PathBuf,
    project: Option<LocatedProject>,
    skipped: SkippedDirectory,
    kind: TreeKind,
}

#[derive(Clone, Copy)]
enum TreeKind {
    Tests,
    Tasks,
}

impl TreeKind {
    const TESTS_ENTRY: &'static str = "<tests>";
    const TASKS_ENTRY: &'static str = "<tasks>";
    fn stays_inside(self) -> bool {
        matches!(self, Self::Tasks)
    }

    fn allows_absent_folder(self) -> bool {
        matches!(self, Self::Tasks)
    }

    fn entry_name(self) -> &'static str {
        match self {
            Self::Tests => Self::TESTS_ENTRY,
            Self::Tasks => Self::TASKS_ENTRY,
        }
    }

    fn shares_source_paths(self) -> bool {
        matches!(self, Self::Tests)
    }

    fn program_file(self, project: &LocatedProject, file: ProgramFile) -> ProgramFile {
        match self {
            Self::Tests => project.placement().trees().test_file(file),
            Self::Tasks => project.placement().trees().task_file(file),
        }
    }
}

impl EntryTree {
    pub const TASKS_ENTRY: &'static str = TreeKind::TASKS_ENTRY;

    pub fn of_tests(path: PathBuf, project: Option<LocatedProject>) -> Self {
        let build_root = project
            .as_ref()
            .map_or(path.as_path(), LocatedProject::root);
        let skipped = SkippedDirectory::for_walk_of(build_root);
        Self {
            path,
            project,
            skipped,
            kind: TreeKind::Tests,
        }
    }

    pub fn of_tasks(project: LocatedProject) -> Self {
        let directory = project.placement().tasks_directory().to_path_buf();
        Self {
            kind: TreeKind::Tasks,
            ..Self::of_tests(directory, Some(project))
        }
    }

    pub fn project(&self) -> Option<&LocatedProject> {
        self.project.as_ref()
    }

    pub fn into_project(self) -> Option<LocatedProject> {
        self.project
    }

    pub fn holds_tasks(&self) -> bool {
        self.project
            .as_ref()
            .is_some_and(|project| project.task_layout().has_tasks())
    }

    pub fn load(&self, terminal: &mut Terminal) -> Result<ModuleOfSources, ExitStatus> {
        if self.is_folder() {
            self.load_directory(terminal)
        } else {
            self.load_file(terminal)
        }
    }

    fn load_file(&self, terminal: &mut Terminal) -> Result<ModuleOfSources, ExitStatus> {
        let file = self.entry_file();
        let named = TreeFile::new(file.module_path().as_str().to_string(), self.path.clone());
        TakenModulePaths::by_the_compiler().refuse(&[named], terminal)?;
        Ok(ModuleOfSources {
            file,
            sources: vec![terminal.sources().load(&self.path)?],
        })
    }

    fn entry_file(&self) -> ProgramFile {
        match &self.project {
            Some(project) => {
                let placement = project.placement();
                placement
                    .trees()
                    .test_file(placement.file_within(self.path.clone()))
            }
            None => ProgramFile::beside_itself(self.path.clone()),
        }
    }

    pub fn module_paths(&self) -> Result<TreeModules, ExitStatus> {
        let (files, walk) = self.walk()?;
        Ok(TreeModules {
            modules: files.into_iter().map(TreeFile::into_module).collect(),
            hidden_folders: walk.hidden_folders(),
            project_files: walk.project_files(),
        })
    }

    fn module_files(&self) -> Result<Vec<TreeFile>, ExitStatus> {
        self.walk().map(|(files, _)| files)
    }

    fn walk(&self) -> Result<(Vec<TreeFile>, TreeWalk<'_>), ExitStatus> {
        let bounds = self.kind.stays_inside().then(|| TreeBounds::of(&self.path));
        let walk = TreeWalk::new(&self.path, &self.skipped, bounds)
            .walk()
            .map_err(|error| {
                UnreadableFile::new(&self.path, &error).report_to(&StandardErrorMessages)
            })?;
        let mut files: Vec<TreeFile> = walk
            .sources()
            .iter()
            .filter_map(|path| Some(TreeFile::new(self.module_path(path)?, path.clone())))
            .collect();
        files.sort();
        match files.iter().find(|file| file.is_unnamable()) {
            Some(file) => Err(file.report_unnamable()),
            None => Ok((files, walk)),
        }
    }

    fn is_folder(&self) -> bool {
        self.path.is_dir() || self.kind.allows_absent_folder()
    }

    fn load_directory(&self, terminal: &mut Terminal) -> Result<ModuleOfSources, ExitStatus> {
        let files = self.module_files()?;
        self.taken_module_paths().refuse(&files, terminal)?;
        let uses: Vec<String> = files
            .iter()
            .map(|file| format!("(use {})", file.module_path()))
            .collect();
        let source = terminal
            .sources()
            .add(self.kind.entry_name(), uses.join("\n"));
        Ok(ModuleOfSources {
            file: self.directory_file(),
            sources: vec![source],
        })
    }

    fn taken_module_paths(&self) -> TakenModulePaths<'_> {
        match &self.project {
            Some(project) => project.placement().taken_module_paths(),
            None => TakenModulePaths::by_the_compiler(),
        }
    }

    fn directory_file(&self) -> ProgramFile {
        let file = ProgramFile::within(self.path.join(self.kind.entry_name()), self.path.clone());
        match &self.project {
            Some(project) => self.kind.program_file(project, file),
            None => file,
        }
    }

    fn module_path(&self, file: &Path) -> Option<String> {
        self.source_module_path(file)
            .or_else(|| ModuleNames::of_source(file.strip_prefix(&self.path).ok()?))
    }

    fn source_module_path(&self, file: &Path) -> Option<String> {
        let project = self
            .project
            .as_ref()
            .filter(|_| self.kind.shares_source_paths())?;
        ProgramFile::relative_module(file, &project.source_root())
    }
}

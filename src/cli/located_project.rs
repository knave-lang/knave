mod entry_layout;
mod file_placement;
mod library_entrance;
mod project_root;
mod task_layout;

use std::path::{Path, PathBuf};

use super::compiler_check::CompilerCheck;
use super::exit_status::ExitStatus;
use super::lending_libraries::LendingLibraries;
use super::library_walk::{LibraryStore, LibraryWalk};
use super::lint_levels::LintLevels;
use super::program_file::ProgramFile;
use super::project_crates::ProjectCrates;
use super::terminal::Terminal;
use super::walk_start::WalkStart;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::language::Language;
use crate::modules::LibraryTable;
use crate::project::{Project, ProjectProblems};
use entry_layout::EntryFacts;
pub use entry_layout::EntryLayout;
use file_placement::FilePlacement;
pub use library_entrance::LibraryEntrance;
pub use project_root::ProjectRoot;
pub use task_layout::{ProjectFacts, TaskLayout};

pub struct LocatedProject {
    root: PathBuf,
    project: Project,
    libraries: LibraryTable,
    crates: ProjectCrates,
    crate_library: Option<String>,
    lending: LendingLibraries,
}

impl LocatedProject {
    pub fn enter_from(
        directory: &Path,
        terminal: &mut Terminal,
        store: impl LibraryStore,
    ) -> Result<Option<Self>, ExitStatus> {
        let Some(root) = ProjectRoot::around(directory) else {
            return Ok(None);
        };
        CompilerCheck::new(terminal).refuse_before_loading(&root)?;
        let located = Self::enter_unchecked(directory, terminal, store);
        CompilerCheck::new(terminal).report(&root)?;
        located
    }

    pub fn enter_unchecked(
        directory: &Path,
        terminal: &mut Terminal,
        store: impl LibraryStore,
    ) -> Result<Option<Self>, ExitStatus> {
        let located = Self::search_from(directory, terminal, store)?;
        if let Some(entered) = &located {
            let levels = LintLevels::of(&entered.project.lints);
            terminal.configure(|settings| settings.leveling_lints_by(levels));
        }
        Ok(located)
    }

    fn search_from(
        directory: &Path,
        terminal: &Terminal,
        store: impl LibraryStore,
    ) -> Result<Option<Self>, ExitStatus> {
        let Some(root) = ProjectRoot::around(directory) else {
            return Ok(None);
        };
        let root = root.shown()?;
        let start = WalkStart {
            root: &root,
            checkout: None,
        };
        Self::walked_from(start, terminal, store).map(Some)
    }

    fn walked_from(
        start: WalkStart<'_>,
        terminal: &Terminal,
        store: impl LibraryStore,
    ) -> Result<Self, ExitStatus> {
        let root = start.root.to_path_buf();
        let mut scratch = terminal.scratch();
        let walked = LibraryWalk::walk(&mut scratch, start, store)?;
        let crates = ProjectCrates::new(
            root.clone(),
            walked.project.name.clone(),
            walked.crates,
            walked.dev_crates,
        );
        Ok(Self {
            root,
            project: walked.project,
            libraries: walked.table,
            crates,
            crate_library: walked.crate_library,
            lending: walked.direct_libraries,
        })
    }

    pub fn lend_libraries(project: Option<&Self>, file: ProgramFile) -> ProgramFile {
        match project {
            Some(project) => file.with_libraries(project.libraries.clone()),
            None => file,
        }
    }

    pub fn name(&self) -> &str {
        &self.project.name
    }

    pub fn entry_layout(&self) -> EntryLayout<'_> {
        let facts = EntryFacts {
            root: &self.root,
            modules: &self.project.modules,
            has_tasks: self.task_layout().has_tasks(),
        };
        EntryLayout::of(facts, self.placement())
    }

    pub fn task_layout(&self) -> TaskLayout<'_> {
        let facts = ProjectFacts {
            project: &self.project,
            lending: &self.lending,
        };
        TaskLayout::of(facts, &self.placement())
    }

    pub fn placement(&self) -> FilePlacement<'_> {
        FilePlacement::new(&self.root, self.source_root(), &self.libraries)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn source_root(&self) -> PathBuf {
        ProgramFile::directory_of(&self.root.join(self.project.modules.entry.path()))
    }

    pub fn refuse_crates(&self, command: &str, terminal: &Terminal) -> Result<(), ExitStatus> {
        if !self.crates.has_crates() {
            return Ok(());
        }
        let mut scratch = terminal.scratch();
        let project = scratch.sources().projects().load(&self.project_file())?;
        let mut problems = Diagnostics::default();
        problems.push(self.crate_use(&project, command));
        Err(scratch.source_report().report(&problems))
    }

    fn crate_use(&self, project: &Project, command: &str) -> Diagnostic {
        if let Some(dependency) = project.dependencies.first() {
            let label = format!("this project uses crate `{}`", dependency.name);
            return ProjectProblems::crates_unavailable_to(command, dependency.name_span, label);
        }
        let library = project
            .libraries
            .iter()
            .find(|library| Some(&library.name) == self.crate_library.as_ref())
            .expect("a project with crates and no crate of its own lists a library that adds them");
        let label = format!("library `{}` adds crates", library.name);
        ProjectProblems::crates_unavailable_to(command, library.name_span, label)
    }

    pub fn project_file(&self) -> PathBuf {
        self.root.join(Language::project_file_name())
    }

    pub fn crates(&self) -> &ProjectCrates {
        &self.crates
    }
}

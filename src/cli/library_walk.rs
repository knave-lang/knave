use std::path::{Path, PathBuf};

use super::exit_status::ExitStatus;
use super::program_file::ProgramFile;
use super::terminal::Terminal;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::language::Language;
use crate::modules::{LibraryRoot, LibraryTable};
use crate::project::{LibraryDependency, Project, ProjectProblems};
use crate::source::Span;

pub struct LibraryWalk<'t> {
    terminal: &'t mut Terminal,
    seen: Vec<SeenLibrary>,
    roots: Vec<LibraryRoot>,
}

struct SeenLibrary {
    name: String,
    root: PathBuf,
    span: Span,
}

impl<'t> LibraryWalk<'t> {
    pub fn table(
        terminal: &'t mut Terminal,
        root: &Path,
        project: &Project,
    ) -> Result<LibraryTable, ExitStatus> {
        let mut walk = Self {
            terminal,
            seen: Vec::new(),
            roots: Vec::new(),
        };
        walk.visit_all(root, project)?;
        Ok(LibraryTable::new(Self::names(project), walk.roots))
    }

    fn visit_all(&mut self, declaring_root: &Path, project: &Project) -> Result<(), ExitStatus> {
        project
            .path_libraries()
            .try_for_each(|(library, path)| self.visit(&declaring_root.join(path), library))
    }

    fn visit(
        &mut self,
        library_root: &Path,
        library: &LibraryDependency,
    ) -> Result<(), ExitStatus> {
        let project_file = library_root.join(Language::project_file_name());
        if !project_file.is_file() {
            let problem = ProjectProblems::library_without_project_file(
                &library.name,
                library.name_span,
                &project_file.display().to_string(),
            );
            return Err(self.report(problem));
        }
        let identity = ProgramFile::canonical(library_root).unwrap_or(library_root.to_path_buf());
        if self.is_seen(library, &identity)? {
            return Ok(());
        }
        self.seen.push(SeenLibrary {
            name: library.name.clone(),
            root: identity,
            span: library.name_span,
        });
        let project = self.terminal.load_project(&project_file)?;
        let root = self.library_root(library, library_root, &project)?;
        self.roots.push(root);
        self.visit_all(library_root, &project)
    }

    fn is_seen(&self, library: &LibraryDependency, identity: &Path) -> Result<bool, ExitStatus> {
        let Some(seen) = self.seen.iter().find(|seen| seen.name == library.name) else {
            return Ok(false);
        };
        if seen.root == identity {
            return Ok(true);
        }
        Err(self.report(ProjectProblems::library_at_two_sources(
            &library.name,
            library.name_span,
            seen.span,
        )))
    }

    fn library_root(
        &self,
        library: &LibraryDependency,
        library_root: &Path,
        project: &Project,
    ) -> Result<LibraryRoot, ExitStatus> {
        let root_file = library_root.join(&project.library_root);
        if !root_file.is_file() {
            let project_file = library_root.join(Language::project_file_name());
            return Err(self.report(ProjectProblems::library_without_root_module(
                &library.name,
                library.name_span,
                &root_file.display().to_string(),
                &project_file.display().to_string(),
            )));
        }
        let source_root = ProgramFile::directory_of(&root_file);
        Ok(LibraryRoot::new(
            library.name.clone(),
            source_root.clone(),
            source_root,
            ProgramFile::file_stem(&root_file),
            Self::names(project),
        ))
    }

    fn report(&self, problem: Diagnostic) -> ExitStatus {
        let mut problems = Diagnostics::default();
        problems.push(problem);
        self.terminal.report(&problems)
    }

    fn names(project: &Project) -> Vec<String> {
        project
            .libraries
            .iter()
            .map(|library| library.name.clone())
            .collect()
    }
}

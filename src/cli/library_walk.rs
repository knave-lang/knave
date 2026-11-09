use std::path::{Path, PathBuf};

use super::exit_status::ExitStatus;
use super::program_file::ProgramFile;
use super::standard_error::StandardError;
use super::terminal::Terminal;
use crate::build::{GitStore, StoreError};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::language::Language;
use crate::modules::{LibraryRoot, LibraryTable};
use crate::project::{LibraryDependency, LibrarySource, Project, ProjectProblems};
use crate::source::Span;

pub struct LibraryWalk<'t> {
    terminal: &'t mut Terminal,
    seen: Vec<SeenLibrary>,
    roots: Vec<LibraryRoot>,
    store: Option<GitStore>,
}

#[derive(Clone, Copy)]
struct Listing<'l> {
    entry: Option<&'l str>,
}

impl Listing<'_> {
    const PROGRAM: Self = Self { entry: None };
}

struct SeenLibrary {
    name: String,
    root: PathBuf,
    span: Span,
}

impl<'t> LibraryWalk<'t> {
    const URL_SCHEME_OR_HOST: char = ':';

    pub fn table(
        terminal: &'t mut Terminal,
        root: &Path,
        project: &Project,
    ) -> Result<LibraryTable, ExitStatus> {
        let mut walk = Self {
            terminal,
            seen: Vec::new(),
            roots: Vec::new(),
            store: None,
        };
        walk.visit_all(root, project, Listing::PROGRAM)?;
        Ok(LibraryTable::new(Self::names(project), walk.roots))
    }

    fn visit_all(
        &mut self,
        declaring_root: &Path,
        project: &Project,
        listing: Listing<'_>,
    ) -> Result<(), ExitStatus> {
        project.libraries.iter().try_for_each(|library| {
            let library_root = self.library_directory(declaring_root, library)?;
            self.visit(&library_root, library, listing)
        })
    }

    fn library_directory(
        &mut self,
        declaring_root: &Path,
        library: &LibraryDependency,
    ) -> Result<PathBuf, ExitStatus> {
        match &library.source {
            LibrarySource::Path(path) => Ok(declaring_root.join(path)),
            LibrarySource::Git { url, commit } => {
                let repository = Self::repository_location(declaring_root, url);
                self.checkout(library, &repository, commit)
            }
        }
    }

    fn repository_location(declaring_root: &Path, url: &str) -> String {
        let is_local_path = !url.contains(Self::URL_SCHEME_OR_HOST) && Path::new(url).is_relative();
        if !is_local_path {
            return url.to_string();
        }
        declaring_root.join(url).display().to_string()
    }

    fn checkout(
        &mut self,
        library: &LibraryDependency,
        url: &str,
        commit: &str,
    ) -> Result<PathBuf, ExitStatus> {
        let store = match self.store.take() {
            Some(store) => store,
            None => GitStore::for_user().map_err(|error| self.report_store(library, error))?,
        };
        let checked_out = store
            .checkout(url, commit)
            .map_err(|error| self.report_store(library, error));
        self.store = Some(store);
        checked_out
    }

    fn report_store(&self, library: &LibraryDependency, error: StoreError) -> ExitStatus {
        match error {
            StoreError::Fetch(reason) => self.report(ProjectProblems::unavailable_git_library(
                &library.name,
                library.name_span,
                &reason,
            )),
            StoreError::NoCacheDirectory => Self::report_environment(
                "no cache directory for git libraries: set `XDG_CACHE_HOME` or `HOME`",
            ),
            StoreError::GitMissing(error) => Self::report_environment(&format!(
                "cannot run `git` to fetch git libraries: {error}"
            )),
            StoreError::Unwritable(path, error) => {
                Self::report_environment(&format!("cannot write `{}`: {error}", path.display()))
            }
        }
    }

    fn report_environment(message: &str) -> ExitStatus {
        StandardError::write_line(format_args!("error: {message}"));
        ExitStatus::Environment
    }

    fn visit(
        &mut self,
        library_root: &Path,
        library: &LibraryDependency,
        listing: Listing<'_>,
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
        self.roots
            .push(root.listed_by(listing.entry.map(str::to_string)));
        let nested = Listing {
            entry: Some(&library.name),
        };
        self.visit_all(library_root, &project, nested)
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

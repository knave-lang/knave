use std::path::{Path, PathBuf};

use super::checkout_bounds::{CheckoutBounds, RepositoryAddress};
use super::exit_status::ExitStatus;
use super::program_file::ProgramFile;
use super::standard_error::StandardError;
use super::terminal::Terminal;
use crate::build::{GitStore, StoreError};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::language::Language;
use crate::modules::{LibraryRoot, LibraryTable};
use crate::project::{
    CrateDependency, CrateSet, LibraryDependency, LibrarySource, Project, ProjectProblems,
};
use crate::source::Span;

pub struct LibraryWalk<'t> {
    terminal: &'t mut Terminal,
    seen: Vec<SeenLibrary>,
    roots: Vec<LibraryRoot>,
    store: Option<GitStore>,
    crates: CrateSet,
}

pub struct WalkedLibraries {
    pub table: LibraryTable,
    pub crates: Vec<CrateDependency>,
}

#[derive(Clone, Copy)]
struct Listing<'l> {
    entry: Option<&'l str>,
    checkout: Option<&'l CheckoutBounds>,
}

impl Listing<'_> {
    const PROGRAM: Self = Self {
        entry: None,
        checkout: None,
    };
}

struct VisitedLibrary<'v> {
    root: &'v Path,
    library: &'v LibraryDependency,
    project: &'v Project,
    checkout: Option<&'v CheckoutBounds>,
}

struct SeenLibrary {
    name: String,
    root: PathBuf,
    span: Span,
}

impl<'t> LibraryWalk<'t> {
    const LIB_KEY: &'static str = ":lib";
    const PATH_KEY: &'static str = ":path";
    const GIT_KEY: &'static str = ":git";

    pub fn walk(
        terminal: &'t mut Terminal,
        root: &Path,
        project: &Project,
    ) -> Result<WalkedLibraries, ExitStatus> {
        let mut walk = Self {
            terminal,
            seen: Vec::new(),
            roots: Vec::new(),
            store: None,
            crates: CrateSet::default(),
        };
        walk.add_crates(root, project)?;
        walk.visit_all(root, project, Listing::PROGRAM)?;
        Ok(WalkedLibraries {
            table: LibraryTable::new(Self::names(project), walk.roots),
            crates: walk.crates.into_crates(),
        })
    }

    fn add_crates(&mut self, root: &Path, project: &Project) -> Result<(), ExitStatus> {
        for dependency in &project.dependencies {
            let resolved = ProgramFile::resolved_crate(dependency, root);
            if let Err(problem) = self.crates.add(resolved) {
                return Err(self.report(problem));
            }
        }
        Ok(())
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
        match RepositoryAddress::of(url) {
            RepositoryAddress::Remote(remote) => remote.to_string(),
            RepositoryAddress::Local(path) => declaring_root.join(path).display().to_string(),
        }
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
        let project_file = self.project_file_of(library_root, library)?;
        if !self.is_first_visit(library_root, library)? {
            return Ok(());
        }
        let project = self.terminal.load_project(&project_file)?;
        let own_checkout = Self::own_checkout(library_root, library);
        let visited = VisitedLibrary {
            root: library_root,
            library,
            project: &project,
            checkout: own_checkout.as_ref().or(listing.checkout),
        };
        self.record(&visited, listing.entry)?;
        let nested = Listing {
            entry: Some(&library.name),
            checkout: visited.checkout,
        };
        self.visit_all(library_root, &project, nested)
    }

    fn project_file_of(
        &self,
        library_root: &Path,
        library: &LibraryDependency,
    ) -> Result<PathBuf, ExitStatus> {
        let project_file = library_root.join(Language::project_file_name());
        if project_file.is_file() {
            return Ok(project_file);
        }
        Err(self.report(ProjectProblems::library_without_project_file(
            &library.name,
            library.name_span,
            &project_file.display().to_string(),
        )))
    }

    fn is_first_visit(
        &mut self,
        library_root: &Path,
        library: &LibraryDependency,
    ) -> Result<bool, ExitStatus> {
        let identity = ProgramFile::canonical(library_root).unwrap_or(library_root.to_path_buf());
        if self.is_seen(library, &identity)? {
            return Ok(false);
        }
        self.seen.push(SeenLibrary {
            name: library.name.clone(),
            root: identity,
            span: library.name_span,
        });
        Ok(true)
    }

    fn own_checkout(library_root: &Path, library: &LibraryDependency) -> Option<CheckoutBounds> {
        let fetched = matches!(library.source, LibrarySource::Git { .. });
        fetched.then(|| CheckoutBounds::of(library_root))
    }

    fn record(
        &mut self,
        visited: &VisitedLibrary<'_>,
        lister: Option<&str>,
    ) -> Result<(), ExitStatus> {
        if let Some(bounds) = visited.checkout {
            self.keep_inside_checkout(bounds, visited.root, visited.library, visited.project)?;
        }
        self.add_crates(visited.root, visited.project)?;
        let root = self.library_root(visited.library, visited.root, visited.project)?;
        let root = match visited.checkout {
            Some(bounds) => root.within_checkout(bounds.root().to_path_buf()),
            None => root,
        };
        self.roots.push(root.listed_by(lister.map(str::to_string)));
        Ok(())
    }

    fn keep_inside_checkout(
        &self,
        bounds: &CheckoutBounds,
        library_root: &Path,
        library: &LibraryDependency,
        project: &Project,
    ) -> Result<(), ExitStatus> {
        if !bounds.contains(&library_root.join(&project.library_root)) {
            return Err(self.report(ProjectProblems::path_outside_checkout(
                &library.name,
                Self::LIB_KEY,
                library.name_span,
            )));
        }
        let outside = project
            .libraries
            .iter()
            .find_map(|nested| Self::reaching_outside(bounds, library_root, nested));
        if let Some((nested, key)) = outside {
            return Err(self.report(ProjectProblems::path_outside_checkout(
                &nested.name,
                key,
                nested.name_span,
            )));
        }
        match Self::crate_outside(bounds, library_root, project) {
            Some(dependency) => Err(self.report(ProjectProblems::crate_path_outside_checkout(
                &dependency.name,
                dependency.name_span,
            ))),
            None => Ok(()),
        }
    }

    fn crate_outside<'p>(
        bounds: &CheckoutBounds,
        library_root: &Path,
        project: &'p Project,
    ) -> Option<&'p CrateDependency> {
        project.dependencies.iter().find(|dependency| {
            dependency
                .path
                .as_ref()
                .is_some_and(|path| !bounds.contains(&library_root.join(path)))
        })
    }

    fn reaching_outside<'d>(
        bounds: &CheckoutBounds,
        library_root: &Path,
        library: &'d LibraryDependency,
    ) -> Option<(&'d LibraryDependency, &'static str)> {
        let (path, key) = match &library.source {
            LibrarySource::Path(path) => (path.clone(), Self::PATH_KEY),
            LibrarySource::Git { url, .. } => match RepositoryAddress::of(url) {
                RepositoryAddress::Remote(_) => return None,
                RepositoryAddress::Local(path) => (path, Self::GIT_KEY),
            },
        };
        (!bounds.contains(&library_root.join(path))).then_some((library, key))
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

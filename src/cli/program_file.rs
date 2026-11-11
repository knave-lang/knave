use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::exit_status::ExitStatus;
use super::standard_error::StandardError;
use crate::core::ModulePath;
use crate::language::Language;
use crate::modules::{LibraryTable, ModuleSources, SearchScope};
use crate::project::CrateDependency;

#[derive(Clone)]
pub struct ProgramFile {
    path: PathBuf,
    root: PathBuf,
    fallback_root: Option<PathBuf>,
    scope: SearchScope,
    module: ModulePath,
    libraries: LibraryTable,
}

impl ProgramFile {
    const CURRENT_DIRECTORY: &str = ".";
    const MODULE_SEPARATOR: &str = "/";

    pub fn beside_itself(path: PathBuf) -> Self {
        let root = Self::directory_of(&path);
        Self::within(path, root)
    }

    pub fn within(path: PathBuf, root: PathBuf) -> Self {
        let module = Self::relative_module(&path, &root).unwrap_or_else(|| Self::file_stem(&path));
        Self {
            path,
            root,
            fallback_root: None,
            scope: SearchScope::default(),
            module: ModulePath::new(module),
            libraries: LibraryTable::default(),
        }
    }

    pub fn inside_or_beside(path: PathBuf, root: PathBuf) -> Self {
        match Self::relative_module(&path, &root) {
            Some(_) => Self::within(path, root),
            None => Self::beside_itself(path),
        }
    }

    pub fn with_fallback_root(self, fallback_root: PathBuf) -> Self {
        Self {
            fallback_root: Some(fallback_root),
            ..self
        }
    }

    pub fn within_scope(self, scope: SearchScope) -> Self {
        Self { scope, ..self }
    }

    pub fn with_libraries(self, libraries: LibraryTable) -> Self {
        Self { libraries, ..self }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn module_sources(&self) -> ModuleSources {
        let sources = ModuleSources::new(self.root.clone(), self.root.clone())
            .with_libraries(self.libraries.clone())
            .within(self.scope.clone());
        match &self.fallback_root {
            Some(fallback_root) => sources.with_fallback_root(fallback_root.clone()),
            None => sources,
        }
    }

    pub fn module_path(&self) -> ModulePath {
        self.module.clone()
    }

    pub fn module_name(relative: &Path) -> Option<String> {
        let without_extension = relative.with_extension("");
        let parts: Vec<&str> = without_extension
            .components()
            .map(|component| component.as_os_str().to_str())
            .collect::<Option<_>>()?;
        Some(parts.join(Self::MODULE_SEPARATOR))
    }

    pub fn is_project_file(path: &Path) -> bool {
        path.file_name()
            .is_some_and(|name| *name == *Language::project_file_name())
    }

    pub fn report_unwritable(path: &Path, error: &io::Error) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: cannot write `{}`: {error}",
            path.display()
        ));
        ExitStatus::UsageError
    }

    pub fn directory_of(path: &Path) -> PathBuf {
        path.parent().map(Path::to_path_buf).unwrap_or_default()
    }

    pub fn relative_module(path: &Path, root: &Path) -> Option<String> {
        let directory = Self::canonical(&Self::directory_of(path))?;
        let relative_directory = directory.strip_prefix(Self::canonical(root)?).ok()?;
        Self::module_name(&relative_directory.join(path.file_name()?))
    }

    pub fn resolved_crate(dependency: &CrateDependency, root: &Path) -> CrateDependency {
        let root = Self::canonical(root).unwrap_or_else(|| root.to_path_buf());
        let mut resolved = dependency.resolved_from(&root);
        resolved.path = resolved
            .path
            .map(|path| Self::canonical(&path).unwrap_or(path));
        resolved
    }

    pub fn canonical(path: &Path) -> Option<PathBuf> {
        fs::canonicalize(Path::new(Self::CURRENT_DIRECTORY).join(path)).ok()
    }

    pub fn file_stem(path: &Path) -> String {
        path.file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}

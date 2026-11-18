use std::path::{Path, PathBuf};

use super::local_files::LocalFiles;
use super::local_root::LocalRoot;
use super::search_scope::SearchScope;
use super::unlisted_library::UnlistedLibrary;
use super::visible_library::{LibraryNaming, VisibleLibrary};
use crate::core::ModulePath;
use crate::language::Language;

#[derive(Clone)]
pub struct LibraryRoot {
    name: String,
    source_root: PathBuf,
    display_root: PathBuf,
    root_module: String,
    visible: Vec<String>,
    lister: Option<String>,
    checkout: Option<PathBuf>,
}

#[derive(Clone, Default)]
pub struct LibraryTable {
    program_visible: Vec<String>,
    libraries: Vec<LibraryRoot>,
    library_section: String,
}

pub struct LibraryFile {
    pub path: PathBuf,
    pub display: PathBuf,
}

impl LibraryRoot {
    pub fn new(
        name: String,
        source_root: PathBuf,
        display_root: PathBuf,
        root_module: String,
        visible: Vec<String>,
    ) -> Self {
        Self {
            name,
            source_root,
            display_root,
            root_module,
            visible,
            lister: None,
            checkout: None,
        }
    }

    pub fn within_checkout(self, checkout: PathBuf) -> Self {
        Self {
            checkout: Some(checkout),
            ..self
        }
    }

    pub fn listed_by(self, lister: Option<String>) -> Self {
        Self { lister, ..self }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn lister(&self) -> Option<&str> {
        self.lister.as_deref()
    }

    pub fn checkout(&self) -> Option<&std::path::Path> {
        self.checkout.as_deref()
    }

    fn module_path_under(&self, inner_module: &str) -> ModulePath {
        ModulePath::new(format!(
            "{}{}{inner_module}",
            self.name,
            ModulePath::SEPARATOR
        ))
    }

    fn file(&self, inner_module: &str) -> LibraryFile {
        let file_name = format!("{inner_module}.{}", Language::EXTENSION);
        LibraryFile {
            path: self.source_root.join(&file_name),
            display: self.display_root.join(&file_name),
        }
    }
}

impl LibraryNaming for LibraryRoot {
    fn name(&self) -> &str {
        &self.name
    }

    fn module_path_of(&self, inner: &Path) -> Option<ModulePath> {
        let inner = ModulePath::of_file(inner)?;
        if inner.as_str() == self.root_module {
            return Some(ModulePath::new(self.name.as_str()));
        }
        Some(self.module_path_under(inner.as_str()))
    }
}

impl LibraryTable {
    const UNSCOPED: SearchScope = SearchScope::Program;

    pub fn new(
        program_visible: Vec<String>,
        libraries: Vec<LibraryRoot>,
        library_section: String,
    ) -> Self {
        Self {
            program_visible,
            libraries,
            library_section,
        }
    }

    pub fn resolve(&self, owner: Option<&str>, written: &ModulePath) -> ModulePath {
        match self.named(owner) {
            Some(library) if !self.sees_library(owner, written) => {
                library.module_path_under(written.as_str())
            }
            _ => written.clone(),
        }
    }

    pub fn unlisted_library(
        &self,
        owner: Option<&str>,
        written: &ModulePath,
    ) -> Option<UnlistedLibrary<'_>> {
        if owner.is_some() {
            return None;
        }
        let root = self.owner_of(written)?;
        (!self.sees_library(None, written)).then_some(UnlistedLibrary {
            name: root.name(),
            lister: root.lister(),
            section: &self.library_section,
        })
    }

    pub fn names_library(&self, owner: Option<&str>, written: &ModulePath) -> bool {
        self.sees_library(owner, written)
    }

    pub(super) fn local_files<'s>(
        &'s self,
        owner: Option<&str>,
        program_roots: Vec<LocalRoot<'s>>,
        compiler_claims: impl Fn(&ModulePath) -> bool + 's,
    ) -> LocalFiles<'s> {
        let library = self.named(owner);
        let other_libraries = self
            .libraries
            .iter()
            .filter(|other| library.is_none_or(|own| own.name != other.name))
            .map(|other| VisibleLibrary {
                library: other,
                root: LocalRoot::primary(&other.source_root, &other.display_root, &Self::UNSCOPED),
                visible: self.sees(owner, &other.name),
                section: &self.library_section,
            })
            .collect();
        match library {
            Some(library) => {
                let root = LocalRoot::primary(
                    &library.source_root,
                    &library.display_root,
                    &Self::UNSCOPED,
                );
                LocalFiles::new(vec![root], other_libraries, move |path| {
                    compiler_claims(path)
                        || self.sees_library(Some(&library.name), path)
                        || path.as_str() == library.root_module
                })
            }
            None => LocalFiles::new(program_roots, other_libraries, move |path| {
                compiler_claims(path) || self.owner_of(path).is_some()
            }),
        }
    }

    fn named(&self, owner: Option<&str>) -> Option<&LibraryRoot> {
        let owner = owner?;
        self.libraries.iter().find(|library| library.name == owner)
    }

    fn sees_library(&self, owner: Option<&str>, written: &ModulePath) -> bool {
        self.sees(owner, Self::head(written))
    }

    fn sees(&self, owner: Option<&str>, library: &str) -> bool {
        let visible = self
            .named(owner)
            .map_or(&self.program_visible, |named| &named.visible);
        visible.iter().any(|name| name == library)
    }

    pub fn file(&self, path: &ModulePath) -> Option<LibraryFile> {
        let library = self.owner_of(path)?;
        let inner = path
            .as_str()
            .strip_prefix(library.name.as_str())?
            .strip_prefix(ModulePath::SEPARATOR);
        Some(library.file(inner.unwrap_or(&library.root_module)))
    }

    pub fn owner_of(&self, path: &ModulePath) -> Option<&LibraryRoot> {
        self.libraries
            .iter()
            .find(|library| library.name == Self::head(path))
    }

    pub fn head(path: &ModulePath) -> &str {
        path.as_str()
            .split(ModulePath::SEPARATOR)
            .next()
            .unwrap_or_default()
    }
}

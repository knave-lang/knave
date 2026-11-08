use std::path::PathBuf;

use crate::core::ModulePath;
use crate::language::Language;

#[derive(Clone)]
pub struct LibraryRoot {
    name: String,
    source_root: PathBuf,
    display_root: PathBuf,
    root_module: String,
    visible: Vec<String>,
}

#[derive(Clone, Default)]
pub struct LibraryTable {
    program_visible: Vec<String>,
    libraries: Vec<LibraryRoot>,
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
        }
    }

    fn file(&self, inner_module: &str) -> LibraryFile {
        let file_name = format!("{inner_module}.{}", Language::EXTENSION);
        LibraryFile {
            path: self.source_root.join(&file_name),
            display: self.display_root.join(&file_name),
        }
    }
}

impl LibraryTable {
    const SEPARATOR: char = '/';

    pub fn new(program_visible: Vec<String>, libraries: Vec<LibraryRoot>) -> Self {
        Self {
            program_visible,
            libraries,
        }
    }

    pub fn resolve(&self, owner: Option<&str>, written: &ModulePath) -> ModulePath {
        match self.named(owner) {
            Some(library) if !self.sees_library(owner, written) => {
                ModulePath::new(format!("{}{}{written}", library.name, Self::SEPARATOR))
            }
            _ => written.clone(),
        }
    }

    pub fn unlisted_library(&self, owner: Option<&str>, written: &ModulePath) -> Option<&str> {
        if owner.is_some() {
            return None;
        }
        let library = self.owner_of(written)?;
        (!self.sees_library(None, written)).then_some(library.name.as_str())
    }

    pub fn lister_of(&self, name: &str) -> Option<&str> {
        self.libraries
            .iter()
            .find(|library| library.visible.iter().any(|visible| visible == name))
            .map(|library| library.name.as_str())
    }

    pub fn names_library(&self, owner: Option<&str>, written: &ModulePath) -> bool {
        self.sees_library(owner, written)
    }

    fn named(&self, owner: Option<&str>) -> Option<&LibraryRoot> {
        let owner = owner?;
        self.libraries.iter().find(|library| library.name == owner)
    }

    fn sees_library(&self, owner: Option<&str>, written: &ModulePath) -> bool {
        let visible = self
            .named(owner)
            .map_or(&self.program_visible, |library| &library.visible);
        visible.iter().any(|name| name == Self::head(written))
    }

    pub fn file(&self, path: &ModulePath) -> Option<LibraryFile> {
        let library = self.owner_of(path)?;
        let inner = path
            .as_str()
            .strip_prefix(library.name.as_str())?
            .strip_prefix(Self::SEPARATOR);
        Some(library.file(inner.unwrap_or(&library.root_module)))
    }

    pub fn owner_of(&self, path: &ModulePath) -> Option<&LibraryRoot> {
        self.libraries
            .iter()
            .find(|library| library.name == Self::head(path))
    }

    pub fn head(path: &ModulePath) -> &str {
        path.as_str()
            .split(Self::SEPARATOR)
            .next()
            .unwrap_or_default()
    }
}

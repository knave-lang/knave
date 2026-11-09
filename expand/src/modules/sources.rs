use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::libraries::{LibraryRoot, LibraryTable};
use crate::core::ModulePath;
use crate::language::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinModule {
    Kernel,
    Code,
}

pub enum ModuleSource {
    Builtin(BuiltinModule),
    Text { name: String, text: String },
}

pub struct LocalModuleClash<'s> {
    pub local: PathBuf,
    pub library: &'s str,
    pub lister: &'s str,
}

pub struct ModuleSources {
    root: PathBuf,
    display_root: PathBuf,
    libraries: LibraryTable,
}

impl ModuleSources {
    pub const KERNEL: &str = "kernel";
    pub const CODE: &str = "code";
    pub const PRELUDE: &str = "prelude";
    pub const TEST: &str = "prelude/test";
    pub const PROJECT_SHAPES: &str = "prelude/project-file";
    const PRELUDE_DIRECTORY: &str = "prelude/";
    const INTERNAL_DIRECTORY: &str = "prelude/internal/";
    const BYTE_ORDER_MARK: char = '\u{feff}';
    const PRELUDE_FILES: [(&str, &str); 24] = [
        ("prelude", include_str!("../../prelude/prelude.rslisp")),
        (
            "prelude/internal/macros",
            include_str!("../../prelude/internal/macros.rslisp"),
        ),
        (
            "prelude/macros",
            include_str!("../../prelude/macros.rslisp"),
        ),
        ("prelude/logic", include_str!("../../prelude/logic.rslisp")),
        (
            "prelude/internal/syntax",
            include_str!("../../prelude/internal/syntax.rslisp"),
        ),
        (
            "prelude/syntax",
            include_str!("../../prelude/syntax.rslisp"),
        ),
        (
            "prelude/number",
            include_str!("../../prelude/number.rslisp"),
        ),
        ("prelude/text", include_str!("../../prelude/text.rslisp")),
        (
            "prelude/struct",
            include_str!("../../prelude/struct.rslisp"),
        ),
        (
            "prelude/definers",
            include_str!("../../prelude/definers.rslisp"),
        ),
        (
            "prelude/ownership",
            include_str!("../../prelude/ownership.rslisp"),
        ),
        ("prelude/types", include_str!("../../prelude/types.rslisp")),
        ("prelude/pair", include_str!("../../prelude/pair.rslisp")),
        ("prelude/assoc", include_str!("../../prelude/assoc.rslisp")),
        ("prelude/place", include_str!("../../prelude/place.rslisp")),
        ("prelude/vec", include_str!("../../prelude/vec.rslisp")),
        (
            "prelude/iterator",
            include_str!("../../prelude/iterator.rslisp"),
        ),
        (
            "prelude/option",
            include_str!("../../prelude/option.rslisp"),
        ),
        (
            "prelude/result",
            include_str!("../../prelude/result.rslisp"),
        ),
        ("prelude/io", include_str!("../../prelude/io.rslisp")),
        ("prelude/map", include_str!("../../prelude/map.rslisp")),
        (
            "prelude/sequence",
            include_str!("../../prelude/sequence.rslisp"),
        ),
        ("prelude/test", include_str!("../../prelude/test.rslisp")),
        (
            "prelude/project-file",
            include_str!("../../prelude/project-file.rslisp"),
        ),
    ];

    pub fn new(root: PathBuf, display_root: PathBuf) -> Self {
        Self {
            root,
            display_root,
            libraries: LibraryTable::default(),
        }
    }

    pub fn with_libraries(self, libraries: LibraryTable) -> Self {
        Self { libraries, ..self }
    }

    pub fn resolve(&self, owner: Option<&str>, written: &ModulePath) -> ModulePath {
        if !Self::is_written(written) {
            return written.clone();
        }
        self.libraries.resolve(owner, written)
    }

    pub fn library_owner<'p>(&self, path: &'p ModulePath) -> Option<&'p str> {
        self.libraries
            .owner_of(path)
            .map(|_| LibraryTable::head(path))
    }

    pub fn unlisted_library(&self, owner: Option<&str>, written: &ModulePath) -> Option<&str> {
        self.libraries
            .unlisted_library(owner, written)
            .map(LibraryRoot::name)
    }

    pub fn local_module_beside_library(
        &self,
        owner: Option<&str>,
        written: &ModulePath,
    ) -> Option<LocalModuleClash<'_>> {
        let library = self.libraries.unlisted_library(owner, written)?;
        Some(LocalModuleClash {
            local: self.local_module(written)?,
            library: library.name(),
            lister: library.lister()?,
        })
    }

    pub fn hidden_local_module(
        &self,
        owner: Option<&str>,
        written: &ModulePath,
    ) -> Option<PathBuf> {
        if owner.is_some() || !self.libraries.names_library(owner, written) {
            return None;
        }
        self.local_module(written)
    }

    fn local_module(&self, written: &ModulePath) -> Option<PathBuf> {
        let head = LibraryTable::head(written);
        let file = PathBuf::from(format!("{head}.{}", Language::EXTENSION));
        [file, PathBuf::from(head)]
            .into_iter()
            .find(|local| self.root.join(local).exists())
            .map(|local| self.display_root.join(local))
    }

    pub fn is_prelude(path: &ModulePath) -> bool {
        path.as_str() == Self::PRELUDE || path.as_str().starts_with(Self::PRELUDE_DIRECTORY)
    }

    pub fn is_internal(path: &ModulePath) -> bool {
        path.as_str().starts_with(Self::INTERNAL_DIRECTORY) && Self::is_embedded_prelude(path)
    }

    pub fn is_embedded_prelude(path: &ModulePath) -> bool {
        Self::PRELUDE_FILES
            .iter()
            .any(|(module, _)| *module == path.as_str())
    }

    pub fn is_written(path: &ModulePath) -> bool {
        !Self::is_prelude(path) && !Self::is_compiler_provided(path)
    }

    pub fn is_reserved_library(name: &str) -> bool {
        [Self::PRELUDE, Self::KERNEL, Self::CODE].contains(&name)
    }

    pub fn is_reserved(path: &ModulePath) -> bool {
        path.as_str() == Language::PROJECT_FILE_STEM
    }

    pub fn is_compiler_provided(path: &ModulePath) -> bool {
        [Self::KERNEL, Self::CODE].contains(&path.as_str())
    }

    pub fn relative_name<'n>(&self, name: &'n str) -> &'n str {
        Path::new(name)
            .strip_prefix(&self.display_root)
            .ok()
            .and_then(Path::to_str)
            .unwrap_or(name)
    }

    pub fn read_file(path: &Path) -> io::Result<String> {
        fs::read_to_string(path).map(Self::without_byte_order_mark)
    }

    pub fn read_standard_input() -> io::Result<String> {
        io::read_to_string(io::stdin()).map(Self::without_byte_order_mark)
    }

    fn without_byte_order_mark(mut text: String) -> String {
        if text.starts_with(Self::BYTE_ORDER_MARK) {
            text.drain(..Self::BYTE_ORDER_MARK.len_utf8());
        }
        text
    }

    pub fn find(&self, path: &ModulePath) -> Option<ModuleSource> {
        self.builtin(path)
            .or_else(|| self.prelude_file(path))
            .or_else(|| self.written_file(path))
    }

    fn builtin(&self, path: &ModulePath) -> Option<ModuleSource> {
        match path.as_str() {
            Self::KERNEL => Some(ModuleSource::Builtin(BuiltinModule::Kernel)),
            Self::CODE => Some(ModuleSource::Builtin(BuiltinModule::Code)),
            _ => None,
        }
    }

    fn prelude_file(&self, path: &ModulePath) -> Option<ModuleSource> {
        Self::PRELUDE_FILES
            .iter()
            .find(|(module, _)| *module == path.as_str())
            .map(|(_, text)| ModuleSource::Text {
                name: Self::file_name(path),
                text: (*text).to_string(),
            })
    }

    fn written_file(&self, path: &ModulePath) -> Option<ModuleSource> {
        match self.libraries.owner_of(path) {
            Some(_) => self.library_file(path),
            None => self.user_file(path),
        }
    }

    pub fn module_outside_checkout(&self, path: &ModulePath) -> Option<PathBuf> {
        let checkout = self.libraries.checkout_of(path)?;
        let file = self.libraries.file(path)?;
        let resolved = fs::canonicalize(&file.path).ok()?;
        (!resolved.starts_with(checkout)).then_some(file.display)
    }

    fn library_file(&self, path: &ModulePath) -> Option<ModuleSource> {
        let file = self.libraries.file(path)?;
        let text = Self::read_file(&file.path).ok()?;
        Some(ModuleSource::Text {
            name: file.display.display().to_string(),
            text,
        })
    }

    fn user_file(&self, path: &ModulePath) -> Option<ModuleSource> {
        let file_name = Self::file_name(path);
        let text = Self::read_file(&self.root.join(&file_name)).ok()?;
        Some(ModuleSource::Text {
            name: self.display_root.join(&file_name).display().to_string(),
            text,
        })
    }

    fn file_name(path: &ModulePath) -> String {
        format!("{path}.{}", Language::EXTENSION)
    }
}

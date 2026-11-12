use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use super::libraries::{LibraryRoot, LibraryTable};
use super::located_module::LocatedModule;
use super::module_source::ModuleSource;
use super::search_scope::SearchScope;
use crate::core::{BuiltinModule, ModulePath};
use crate::language::Language;

pub struct LocalModuleClash<'s> {
    pub local: PathBuf,
    pub library: &'s str,
    pub lister: &'s str,
}

pub struct HiddenSourceModule {
    pub hiding: PathBuf,
    pub hidden: PathBuf,
    pub directory_name: &'static str,
}

pub struct ModuleSources {
    root: PathBuf,
    display_root: PathBuf,
    fallback_root: Option<PathBuf>,
    scope: SearchScope,
    libraries: LibraryTable,
}

impl ModuleSources {
    pub const KERNEL: &str = "kernel";
    pub const CODE: &str = "code";
    pub const PRELUDE_CODE: &str = "prelude/internal/code";
    pub const PRELUDE: &str = "prelude";
    pub const TEST: &str = "prelude/test";
    pub const LIBRARY_EXAMPLES: &str = "prelude/library-examples";
    pub const TASK: &str = "prelude/task";
    pub const PROJECT_SHAPES: &str = "prelude/project-file";
    pub const DOCUMENTATION: &str = "prelude/documentation";
    pub const DOCSTRINGS: &str = "prelude/internal/docstrings";
    const PRELUDE_DIRECTORY: &str = "prelude/";
    const INTERNAL_DIRECTORY: &str = "prelude/internal/";
    const BYTE_ORDER_MARK: char = '\u{feff}';
    const PRELUDE_FILES: [(&str, &str); 27] = [
        ("prelude", include_str!("../../prelude/prelude.knv")),
        (
            "prelude/internal/macros",
            include_str!("../../prelude/internal/macros.knv"),
        ),
        ("prelude/macros", include_str!("../../prelude/macros.knv")),
        ("prelude/logic", include_str!("../../prelude/logic.knv")),
        (
            "prelude/internal/syntax",
            include_str!("../../prelude/internal/syntax.knv"),
        ),
        ("prelude/syntax", include_str!("../../prelude/syntax.knv")),
        ("prelude/number", include_str!("../../prelude/number.knv")),
        ("prelude/text", include_str!("../../prelude/text.knv")),
        ("prelude/struct", include_str!("../../prelude/struct.knv")),
        (
            "prelude/definers",
            include_str!("../../prelude/definers.knv"),
        ),
        (
            "prelude/ownership",
            include_str!("../../prelude/ownership.knv"),
        ),
        ("prelude/types", include_str!("../../prelude/types.knv")),
        ("prelude/pair", include_str!("../../prelude/pair.knv")),
        ("prelude/assoc", include_str!("../../prelude/assoc.knv")),
        ("prelude/place", include_str!("../../prelude/place.knv")),
        ("prelude/vec", include_str!("../../prelude/vec.knv")),
        (
            "prelude/iterator",
            include_str!("../../prelude/iterator.knv"),
        ),
        ("prelude/option", include_str!("../../prelude/option.knv")),
        ("prelude/result", include_str!("../../prelude/result.knv")),
        ("prelude/io", include_str!("../../prelude/io.knv")),
        ("prelude/map", include_str!("../../prelude/map.knv")),
        (
            "prelude/sequence",
            include_str!("../../prelude/sequence.knv"),
        ),
        (
            "prelude/internal/docstrings",
            include_str!("../../prelude/internal/docstrings.knv"),
        ),
        ("prelude/test", include_str!("../../prelude/test.knv")),
        ("prelude/task", include_str!("../../prelude/task.knv")),
        (
            "prelude/project-file",
            include_str!("../../prelude/project-file.knv"),
        ),
        (
            "prelude/documentation",
            include_str!("../../prelude/documentation.knv"),
        ),
    ];

    pub fn new(root: PathBuf, display_root: PathBuf) -> Self {
        Self {
            root,
            display_root,
            fallback_root: None,
            scope: SearchScope::default(),
            libraries: LibraryTable::default(),
        }
    }

    pub fn with_fallback_root(self, fallback_root: PathBuf) -> Self {
        Self {
            fallback_root: Some(fallback_root),
            ..self
        }
    }

    pub fn within(self, scope: SearchScope) -> Self {
        Self { scope, ..self }
    }

    pub fn holds_task(&self, file: &str) -> bool {
        self.scope.holds_task(Path::new(file))
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

    pub fn locate(&self, path: &ModulePath) -> Option<LocatedModule> {
        let source = self.find(path)?;
        let owner = self
            .libraries
            .owner_of(path)
            .map(|_| Rc::from(LibraryTable::head(path)));
        Some(LocatedModule { source, owner })
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

    pub fn hidden_source_module(
        &self,
        owner: Option<&str>,
        written: &ModulePath,
    ) -> Option<HiddenSourceModule> {
        let fallback_root = self.fallback_root.as_ref()?;
        if owner.is_some() || self.libraries.owner_of(written).is_some() {
            return None;
        }
        let file_name = Self::file_name(written);
        let hidden = fallback_root.join(&file_name);
        let hides = self.root.join(&file_name).is_file() && hidden.is_file();
        hides.then(|| HiddenSourceModule {
            hiding: self.display_root.join(&file_name),
            hidden,
            directory_name: self.scope.directory_name(),
        })
    }

    pub fn is_written_file(name: &str) -> bool {
        Path::new(name).is_file()
    }

    fn local_module(&self, written: &ModulePath) -> Option<PathBuf> {
        let head = LibraryTable::head(written);
        let file = PathBuf::from(format!("{head}.{}", Language::EXTENSION));
        self.local_roots().find_map(|local_root| {
            [&file, Path::new(head)]
                .into_iter()
                .find(|local| local_root.reaches(local))
                .map(|local| local_root.display_root.join(local))
        })
    }

    fn local_roots(&self) -> impl Iterator<Item = LocalRoot<'_>> {
        let fallback = self.fallback_root.as_deref();
        let primary = LocalRoot {
            root: &self.root,
            display_root: &self.display_root,
            scope: &self.scope,
            role: RootRole::Primary,
        };
        std::iter::once(primary).chain(fallback.map(|root| LocalRoot {
            root,
            display_root: root,
            scope: &self.scope,
            role: RootRole::Fallback,
        }))
    }

    pub fn unreachable_file(&self, path: &ModulePath) -> Option<PathBuf> {
        let file_name = Self::file_name(path);
        self.local_roots()
            .find(|local_root| local_root.holds_unreachable(Path::new(&file_name)))
            .map(|local_root| Self::normalized(&local_root.display_root.join(&file_name)))
    }

    fn normalized(path: &Path) -> PathBuf {
        let mut normalized = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir if normalized.file_name().is_some() => {
                    normalized.pop();
                }
                other => normalized.push(other),
            }
        }
        normalized
    }

    pub fn is_prelude(path: &ModulePath) -> bool {
        path.as_str() == Self::PRELUDE || path.as_str().starts_with(Self::PRELUDE_DIRECTORY)
    }

    pub fn is_internal(path: &ModulePath) -> bool {
        path.as_str().starts_with(Self::INTERNAL_DIRECTORY)
            && (Self::is_embedded_prelude(path) || path.as_str() == Self::PRELUDE_CODE)
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
        [Self::KERNEL, Self::CODE, Self::PRELUDE_CODE].contains(&path.as_str())
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
            Self::PRELUDE_CODE => Some(ModuleSource::Builtin(BuiltinModule::PreludeCode)),
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
        self.local_roots().find_map(|local_root| {
            if !local_root.reaches(Path::new(&file_name)) {
                return None;
            }
            let text = Self::read_file(&local_root.root.join(&file_name)).ok()?;
            Some(ModuleSource::Text {
                name: local_root
                    .display_root
                    .join(&file_name)
                    .display()
                    .to_string(),
                text,
            })
        })
    }

    fn file_name(path: &ModulePath) -> String {
        format!("{path}.{}", Language::EXTENSION)
    }
}

struct LocalRoot<'s> {
    root: &'s Path,
    display_root: &'s Path,
    scope: &'s SearchScope,
    role: RootRole,
}

#[derive(Clone, Copy)]
enum RootRole {
    Primary,
    Fallback,
}

pub struct PublicPrelude;

impl PublicPrelude {
    pub fn paths() -> Vec<String> {
        ModuleSources::PRELUDE_FILES
            .iter()
            .map(|(module, _)| *module)
            .filter(|module| !module.starts_with(ModuleSources::INTERNAL_DIRECTORY))
            .map(String::from)
            .collect()
    }
}

impl LocalRoot<'_> {
    fn reaches(&self, local: &Path) -> bool {
        let path = self.root.join(local);
        path.exists() && self.allows(&path)
    }

    fn holds_unreachable(&self, local: &Path) -> bool {
        let path = self.root.join(local);
        path.exists() && !self.allows(&path)
    }

    fn allows(&self, path: &Path) -> bool {
        match self.role {
            RootRole::Primary => self.scope.primary_reaches(path),
            RootRole::Fallback => self.scope.fallback_reaches(path),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::ModuleSources;
    use crate::core::ModulePath;
    use crate::modules::libraries::{LibraryRoot, LibraryTable};

    struct TestedApp {
        directory: PathBuf,
    }

    impl TestedApp {
        const LIBRARY: &str = "greeting";

        fn new() -> Self {
            Self {
                directory: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../tests/fixtures/libraries/tested-app"),
            }
        }

        fn sources(&self) -> ModuleSources {
            let tests = self.directory.join("shadowing-tests");
            ModuleSources::new(tests.clone(), tests).with_fallback_root(self.directory.join("src"))
        }

        fn sources_with_library(&self) -> ModuleSources {
            let library = LibraryRoot::new(
                Self::LIBRARY.to_string(),
                self.directory.join("src"),
                self.directory.join("src"),
                "main".to_string(),
                Vec::new(),
            );
            let table = LibraryTable::new(vec![Self::LIBRARY.to_string()], vec![library]);
            self.sources().with_libraries(table)
        }

        fn hidden(
            &self,
            sources: &ModuleSources,
            owner: Option<&str>,
            written: &str,
        ) -> Option<PathBuf> {
            sources
                .hidden_source_module(owner, &ModulePath::new(written.to_string()))
                .map(|shadow| shadow.hidden)
        }
    }

    #[test]
    fn a_test_module_hides_the_source_module_of_its_path() {
        let app = TestedApp::new();
        let hidden = app.hidden(&app.sources(), None, "greeting");
        assert_eq!(hidden, Some(app.directory.join("src/greeting.knv")));
    }

    #[test]
    fn a_source_module_without_a_test_module_is_not_hidden() {
        let app = TestedApp::new();
        assert_eq!(app.hidden(&app.sources(), None, "parser"), None);
    }

    #[test]
    fn a_test_module_without_a_source_module_hides_nothing() {
        let app = TestedApp::new();
        assert_eq!(app.hidden(&app.sources(), None, "greets"), None);
    }

    #[test]
    fn a_use_inside_a_library_hides_nothing() {
        let app = TestedApp::new();
        assert_eq!(app.hidden(&app.sources(), Some("hints"), "greeting"), None);
    }

    #[test]
    fn a_library_path_hides_nothing() {
        let app = TestedApp::new();
        assert_eq!(
            app.hidden(&app.sources_with_library(), None, "greeting"),
            None
        );
    }
}

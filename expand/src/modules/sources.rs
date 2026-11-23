use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::claim_rules::ClaimRules;
use super::libraries::LibraryTable;
use super::library_file_problem::LibraryFileProblem;
use super::local_files::LocalFiles;
use super::local_root::{LocalRoot, ReachedFile};
use super::located_module::LocatedModule;
use super::module_source::ModuleSource;
use super::project_folder::ProjectFolder;
use super::real_path::RealPath;
use super::reserved_module_paths::ReservedModulePaths;
use super::search_scope::SearchScope;
use super::unlisted_library::UnlistedLibrary;
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
    pub const TEST: &str = "prelude/test";
    pub const LIBRARY_EXAMPLES: &str = "prelude/library-examples";
    pub const TASK: &str = "prelude/task";
    pub const PROJECT_SHAPES: &str = "prelude/project-file";
    pub const DOCUMENTATION: &str = "prelude/documentation";
    pub const DOCSTRINGS: &str = "prelude/internal/docstrings";
    pub const LINT: &str = "prelude/lint";
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
    // A 28th entry in `PRELUDE_FILES` stops LLVM from unrolling its scan, which costs every
    // command thousands of instructions, so the tool modules live in their own list.
    const TOOL_FILES: [(&str, &str); 2] = [
        (Self::LINT, include_str!("../../prelude/lint.knv")),
        (
            "prelude/internal/links",
            include_str!("../../prelude/internal/links.knv"),
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

    pub fn project_folders(&self, file: &str) -> Vec<ProjectFolder> {
        self.scope.project_folders(Path::new(file))
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
        let owner = self.libraries.owner_of(path).map(|_| Rc::from(path.head()));
        Some(LocatedModule { source, owner })
    }

    pub fn unlisted_library(
        &self,
        owner: Option<&str>,
        written: &ModulePath,
    ) -> Option<UnlistedLibrary<'_>> {
        self.libraries.unlisted_library(owner, written)
    }

    pub fn local_module_beside_library(
        &self,
        owner: Option<&str>,
        written: &ModulePath,
    ) -> Option<LocalModuleClash<'_>> {
        let library = self.libraries.unlisted_library(owner, written)?;
        Some(LocalModuleClash {
            local: self.local_module(written)?,
            library: library.name,
            lister: library.lister?,
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
        let file_name = Language::file_name(written.as_str());
        let hidden = fallback_root.join(&file_name);
        let hides = self.root.join(&file_name).is_file() && hidden.is_file();
        hides.then(|| HiddenSourceModule {
            hiding: self.display_root.join(&file_name),
            hidden,
            directory_name: self.scope.directory_name(),
        })
    }

    fn local_module(&self, written: &ModulePath) -> Option<PathBuf> {
        let head = written.head();
        let file = PathBuf::from(Language::file_name(head));
        self.local_roots().find_map(|local_root| {
            [&file, Path::new(head)]
                .into_iter()
                .find_map(|local| local_root.reached_file(local))
                .map(|reached| reached.shown)
        })
    }

    fn local_roots(&self) -> impl Iterator<Item = LocalRoot<'_>> {
        let primary = LocalRoot::primary(&self.root, &self.display_root, &self.scope);
        let fallback = self.fallback_root.as_deref();
        std::iter::once(primary).chain(fallback.map(|root| LocalRoot::fallback(root, &self.scope)))
    }

    pub fn local_files(&self, owner: Option<Rc<str>>) -> LocalFiles<'_> {
        let claims = ClaimRules {
            program: Self::program_claims,
            library: Self::compiler_claims,
        };
        self.libraries
            .local_files(owner.as_deref(), self.local_roots().collect(), claims)
    }

    fn program_claims(path: &ModulePath) -> bool {
        Self::compiler_claims(path) || ReservedModulePaths::is_reserved(path)
    }

    fn compiler_claims(path: &ModulePath) -> bool {
        !Self::is_written(path)
    }

    pub fn is_prelude(path: &ModulePath) -> bool {
        path.as_str() == ReservedModulePaths::PRELUDE
            || path.as_str().starts_with(Self::PRELUDE_DIRECTORY)
    }

    pub fn is_internal(path: &ModulePath) -> bool {
        path.as_str().starts_with(Self::INTERNAL_DIRECTORY)
            && (Self::is_embedded_prelude(path)
                || path.as_str() == ReservedModulePaths::PRELUDE_CODE)
    }

    pub fn is_embedded_prelude(path: &ModulePath) -> bool {
        EmbeddedModules::contains(path)
    }

    pub fn is_written(path: &ModulePath) -> bool {
        !Self::is_prelude(path) && !ReservedModulePaths::is_compiler_provided(path)
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
            ReservedModulePaths::KERNEL => Some(ModuleSource::Builtin(BuiltinModule::Kernel)),
            ReservedModulePaths::CODE => Some(ModuleSource::Builtin(BuiltinModule::Code)),
            ReservedModulePaths::PRELUDE_CODE => {
                Some(ModuleSource::Builtin(BuiltinModule::PreludeCode))
            }
            _ => None,
        }
    }

    fn prelude_file(&self, path: &ModulePath) -> Option<ModuleSource> {
        let is_named = |(module, _): &&(&str, &str)| *module == path.as_str();
        Self::PRELUDE_FILES
            .iter()
            .find(is_named)
            .or_else(|| Self::TOOL_FILES.iter().find(is_named))
            .map(|(_, text)| ModuleSource::Text {
                name: Language::file_name(path.as_str()),
                text: (*text).to_string(),
                file: None,
            })
    }

    fn written_file(&self, path: &ModulePath) -> Option<ModuleSource> {
        match self.libraries.owner_of(path) {
            Some(_) => self.library_file(path),
            None => self.user_file(path),
        }
    }

    pub fn library_file_problem(&self, path: &ModulePath) -> Option<LibraryFileProblem<'_>> {
        let Some(library) = self.libraries.owner_of(path) else {
            return self.program_file_outside(path);
        };
        let file = self.libraries.file(path)?;
        if !file.path.is_file() {
            return library
                .missing_root_module(path, file.display)
                .map(LibraryFileProblem::MissingRootModule);
        }
        self.outside_checkout(path, &file.path, file.display)
    }

    fn program_file_outside(&self, path: &ModulePath) -> Option<LibraryFileProblem<'_>> {
        let reached = self.reached_files(path).next()?;
        self.outside_checkout(path, &reached.path, reached.shown)
    }

    fn outside_checkout(
        &self,
        path: &ModulePath,
        file: &Path,
        shown: PathBuf,
    ) -> Option<LibraryFileProblem<'_>> {
        let checkout = self.libraries.checkout_of(path)?;
        let resolved = fs::canonicalize(file).ok()?;
        (!resolved.starts_with(checkout)).then_some(LibraryFileProblem::OutsideCheckout(shown))
    }

    fn library_file(&self, path: &ModulePath) -> Option<ModuleSource> {
        let file = self.libraries.file(path)?;
        let text = Self::read_file(&file.path).ok()?;
        Some(ModuleSource::Text {
            name: file.display.display().to_string(),
            text,
            file: fs::canonicalize(&file.path).ok().map(RealPath::new),
        })
    }

    fn user_file(&self, path: &ModulePath) -> Option<ModuleSource> {
        self.reached_files(path).find_map(|reached| {
            let text = Self::read_file(&reached.path).ok()?;
            Some(ModuleSource::Text {
                name: reached.shown.display().to_string(),
                text,
                file: fs::canonicalize(&reached.path).ok().map(RealPath::new),
            })
        })
    }

    fn reached_files(&self, path: &ModulePath) -> impl Iterator<Item = ReachedFile> + '_ {
        let file_name = Language::file_name(path.as_str());
        self.local_roots()
            .filter_map(move |local_root| local_root.reached_file(Path::new(&file_name)))
    }
}

pub struct EmbeddedModules;

impl EmbeddedModules {
    pub fn paths() -> impl Iterator<Item = &'static str> {
        ModuleSources::PRELUDE_FILES
            .iter()
            .chain(&ModuleSources::TOOL_FILES)
            .map(|(module, _)| *module)
    }

    pub fn contains(path: &ModulePath) -> bool {
        Self::paths().any(|module| module == path.as_str())
    }
}

pub struct PublicPrelude;

impl PublicPrelude {
    pub fn paths() -> Vec<String> {
        EmbeddedModules::paths()
            .filter(|module| !module.starts_with(ModuleSources::INTERNAL_DIRECTORY))
            .map(String::from)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{EmbeddedModules, ModuleSources, PublicPrelude};
    use crate::core::ModulePath;
    use crate::language::Language;
    use crate::modules::ModuleSource;
    use crate::modules::libraries::{LibraryRoot, LibraryTable};
    use crate::modules::{LibraryFolder, ProgramLibraries};

    struct TestedApp {
        directory: PathBuf,
    }

    impl TestedApp {
        const LIBRARY: &str = "greeting";
        const LIBRARY_SECTION: &str = "books";

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
                LibraryFolder {
                    project_file: self.directory.join(Language::project_file_name()),
                    source_root: self.directory.join("src"),
                    display_root: self.directory.join("src"),
                },
                "main".to_string(),
                Vec::new(),
            );
            let section = Self::LIBRARY_SECTION.to_string();
            let table = LibraryTable::new(
                ProgramLibraries {
                    visible: vec![Self::LIBRARY.to_string()],
                    checkout: None,
                },
                vec![library],
                section,
            );
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
    fn every_reader_of_the_embedded_files_sees_the_tool_files() {
        let lint = ModulePath::new(ModuleSources::LINT);
        let sources = ModuleSources::new(PathBuf::from("no-such-root"), PathBuf::new());
        assert!(ModuleSources::is_embedded_prelude(&lint));
        assert!(matches!(
            sources.find(&lint),
            Some(ModuleSource::Text { .. })
        ));
        assert!(PublicPrelude::paths().contains(&ModuleSources::LINT.to_string()));
    }

    #[test]
    fn only_a_module_the_compiler_embeds_is_embedded() {
        let embedded = |path: &str| EmbeddedModules::contains(&ModulePath::new(path));
        assert!(embedded("prelude/internal/links"));
        assert!(!embedded("prelude/tests/text"));
        assert!(!embedded("geometry"));
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

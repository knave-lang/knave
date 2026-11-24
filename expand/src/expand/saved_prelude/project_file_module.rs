use rustc_hash::FxHashSet;

use super::lone_prelude_module::LonePreludeModule;
use crate::core::ModulePath;
#[cfg(test)]
use crate::diagnostic::Diagnostics;
use crate::modules::ModuleSources;
use crate::project::{LevelKeys, ProjectRoles, ProjectVocabulary};
use crate::reader::Form;
#[cfg(test)]
use crate::source::SourceMap;

pub struct ProjectFileModule {
    macros: FxHashSet<String>,
    roles: Vec<Form>,
    levels: Vec<Form>,
}

impl ProjectFileModule {
    pub fn expanded() -> Self {
        let path = ModulePath::new(ModuleSources::PROJECT_SHAPES);
        Self::of(&LonePreludeModule::expanded(&path))
    }

    pub fn vocabulary(&self) -> ProjectVocabulary {
        LevelKeys::read(&self.levels)
            .and_then(|levels| ProjectRoles::new(&self.macros).read(&self.roles, levels))
            .unwrap_or_else(|problem| {
                panic!("prelude/project-file states its project roles wrongly: {problem:?}")
            })
    }

    #[cfg(test)]
    fn expanding(
        sources: &mut SourceMap,
        module_sources: ModuleSources,
        path: &ModulePath,
    ) -> Result<Self, Diagnostics> {
        LonePreludeModule::expanding(sources, module_sources, path).map(|module| Self::of(&module))
    }

    fn of(module: &LonePreludeModule) -> Self {
        Self {
            macros: module.macros(),
            roles: module.notes(ProjectRoles::NOTE),
            levels: module.notes(LevelKeys::NOTE),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::ProjectFileModule;
    use crate::core::ModulePath;
    use crate::diagnostic::{DiagnosticCode, Diagnostics};
    use crate::modules::ModuleSources;
    use crate::source::SourceMap;

    struct ProjectFileFixture;

    impl ProjectFileFixture {
        const BROKEN: &str = "../tests/fixtures/expand/broken_project_file";

        fn expanding(module: &str) -> Result<ProjectFileModule, Diagnostics> {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(Self::BROKEN);
            let module_sources = ModuleSources::new(root.clone(), root);
            let mut sources = SourceMap::default();
            ProjectFileModule::expanding(&mut sources, module_sources, &ModulePath::new(module))
        }

        fn codes(failure: Result<ProjectFileModule, Diagnostics>) -> Vec<DiagnosticCode> {
            let Err(problems) = failure else {
                return Vec::new();
            };
            problems
                .in_source_order()
                .into_iter()
                .filter_map(|problem| problem.code().cloned())
                .collect()
        }
    }

    #[test]
    fn a_project_file_module_with_an_error_fails() {
        let codes = ProjectFileFixture::codes(ProjectFileFixture::expanding("project-file"));
        assert_eq!(codes, [DiagnosticCode::UNBOUND_NAME]);
    }

    #[test]
    fn a_project_file_module_that_cannot_load_fails() {
        let codes = ProjectFileFixture::codes(ProjectFileFixture::expanding("absent"));
        assert_eq!(codes, [DiagnosticCode::UNKNOWN_MODULE]);
    }

    #[test]
    fn the_macros_are_the_ones_the_module_defines() {
        let macros = ProjectFileModule::expanded().macros;
        assert!(macros.contains("defproject"));
        assert!(!macros.contains("header-shape"));
    }
}

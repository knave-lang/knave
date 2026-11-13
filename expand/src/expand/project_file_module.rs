use std::path::PathBuf;

use rustc_hash::FxHashSet;

use super::Expander;
use crate::core::{BindingArena, BindingKind, ModuleId, ModulePath};
use crate::diagnostic::{DiagnosticRenderer, Diagnostics};
use crate::modules::ModuleSources;
use crate::project::{ProjectRoles, ProjectVocabulary};
use crate::reader::Form;
use crate::source::{SourceMap, Span};

pub struct ProjectFileModule {
    macros: FxHashSet<String>,
    roles: Vec<Form>,
}

impl ProjectFileModule {
    const ENTRY: &str = "saved-project-roles-entry";

    pub fn expanded() -> Self {
        let mut sources = SourceMap::default();
        let module_sources = ModuleSources::new(PathBuf::new(), PathBuf::new());
        let path = ModulePath::new(ModuleSources::PROJECT_SHAPES);
        Self::expanding(&mut sources, module_sources, &path).unwrap_or_else(|problems| {
            panic!(
                "prelude/project-file expands without problems, but it reports:\n{}",
                DiagnosticRenderer::plain(&sources).render_all(&problems)
            )
        })
    }

    pub fn vocabulary(&self) -> ProjectVocabulary {
        ProjectRoles::new(&self.macros)
            .read(&self.roles)
            .unwrap_or_else(|problem| {
                panic!("prelude/project-file states its project roles wrongly: {problem:?}")
            })
    }

    fn expanding(
        sources: &mut SourceMap,
        module_sources: ModuleSources,
        path: &ModulePath,
    ) -> Result<Self, Diagnostics> {
        let entry = sources.add(Self::ENTRY, "");
        let mut expander = Expander::new(sources, module_sources);
        let loaded = expander.load_module(path, Span::new(entry, 0..0));
        let mut problems = expander.take_diagnostics();
        problems.extend(expander.problems.take_warnings());
        let module = match loaded {
            Ok(module) => module,
            Err(problem) => {
                problems.push(problem);
                return Err(problems);
            }
        };
        problems.into_result(Self {
            macros: Self::macros_of(&expander.tables.bindings, module),
            roles: expander.tables.module_notes.of(module, ProjectRoles::NOTE),
        })
    }

    fn macros_of(bindings: &BindingArena, module: ModuleId) -> FxHashSet<String> {
        bindings
            .ids()
            .map(|id| bindings.get(id))
            .filter(|binding| binding.kind() == BindingKind::Macro(module))
            .map(|binding| binding.name().to_string())
            .collect()
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

use std::path::PathBuf;

use rustc_hash::FxHashSet;

use super::super::{Expander, ExpansionTables};
use crate::core::{BindingKind, ModuleId, ModulePath};
use crate::diagnostic::{DiagnosticRenderer, Diagnostics};
use crate::modules::ModuleSources;
use crate::reader::Form;
use crate::source::{SourceMap, Span};

pub(super) struct LonePreludeModule {
    tables: ExpansionTables,
    module: ModuleId,
}

impl LonePreludeModule {
    const ENTRY: &str = "lone-prelude-module-entry";

    pub(super) fn expanded(path: &ModulePath) -> Self {
        let mut sources = SourceMap::default();
        let module_sources = ModuleSources::new(PathBuf::new(), PathBuf::new());
        Self::expanding(&mut sources, module_sources, path).unwrap_or_else(|problems| {
            panic!(
                "{path} expands without problems, but it reports:\n{}",
                DiagnosticRenderer::plain(&sources).render_all(&problems)
            )
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
            tables: std::mem::take(&mut expander.tables),
            module,
        })
    }

    pub(super) fn notes(&self, key: &str) -> Vec<Form> {
        self.tables.module_notes.of(self.module, key)
    }

    pub(super) fn macros(&self) -> FxHashSet<String> {
        let bindings = &self.tables.bindings;
        bindings
            .ids()
            .map(|id| bindings.get(id))
            .filter(|binding| binding.kind() == BindingKind::Macro(self.module))
            .map(|binding| binding.name().to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::LonePreludeModule;
    use crate::core::ModulePath;
    use crate::diagnostic::{DiagnosticCode, Diagnostics};
    use crate::modules::ModuleSources;
    use crate::source::SourceMap;

    struct LonePreludeFixture;

    impl LonePreludeFixture {
        const BROKEN: &str = "../tests/fixtures/expand/broken_project_file";

        fn expanding(module: &str) -> Result<LonePreludeModule, Diagnostics> {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(Self::BROKEN);
            let module_sources = ModuleSources::new(root.clone(), root);
            let mut sources = SourceMap::default();
            LonePreludeModule::expanding(&mut sources, module_sources, &ModulePath::new(module))
        }

        fn codes(failure: Result<LonePreludeModule, Diagnostics>) -> Vec<DiagnosticCode> {
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
    fn a_lone_prelude_module_with_an_error_fails() {
        let codes = LonePreludeFixture::codes(LonePreludeFixture::expanding("project-file"));
        assert_eq!(codes, [DiagnosticCode::UNBOUND_NAME]);
    }

    #[test]
    fn a_lone_prelude_module_that_cannot_load_fails() {
        let codes = LonePreludeFixture::codes(LonePreludeFixture::expanding("absent"));
        assert_eq!(codes, [DiagnosticCode::UNKNOWN_MODULE]);
    }
}

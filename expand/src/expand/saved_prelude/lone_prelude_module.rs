use std::path::PathBuf;

use rustc_hash::FxHashSet;

use super::super::{Expander, ExpansionTables, MacroCallLookup};
use crate::core::{BindingKind, ModuleId, ModulePath};
use crate::diagnostic::{DiagnosticRenderer, Diagnostics};
use crate::modules::ModuleSources;
use crate::project::WrittenEntries;
use crate::reader::Form;
use crate::source::{SourceMap, Span};

pub(super) struct LonePreludeModule {
    tables: ExpansionTables,
    module: ModuleId,
    sources: SourceMap,
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
        let (tables, module) = Self::expansion(sources, module_sources, path)?;
        Ok(Self {
            tables,
            module,
            sources: std::mem::take(sources),
        })
    }

    fn expansion(
        sources: &mut SourceMap,
        module_sources: ModuleSources,
        path: &ModulePath,
    ) -> Result<(ExpansionTables, ModuleId), Diagnostics> {
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
        problems.into_result((std::mem::take(&mut expander.tables), module))
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

    fn written_call(&self, site: Span) -> Span {
        let call = self.tables.macro_origins.calls.enclosing(site);
        call.map_or(site, |call| call.call_site)
    }
}

impl WrittenEntries for LonePreludeModule {
    fn quoted_call(&self, site: Span) -> String {
        let written = self.written_call(site);
        let text = &self.sources.file(written.source()).text()[written.range()];
        format!("{}: `{text}`", self.sources.location(written))
    }

    fn location(&self, site: Span) -> String {
        self.sources.location(self.written_call(site))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::LonePreludeModule;
    use crate::core::ModulePath;
    use crate::diagnostic::{DiagnosticCode, Diagnostics};
    use crate::modules::{EmbeddedModules, ModuleSources};
    use crate::project::{LintDefaults, WrittenEntries};
    use crate::source::{SourceMap, Span};

    struct LonePreludeFixture;

    impl LonePreludeFixture {
        const BROKEN: &str = "../tests/fixtures/expand/broken_project_file";
        const FORM_GAP: &str = "\n\n";

        fn lint() -> LonePreludeModule {
            LonePreludeModule::expanded(&ModulePath::new(EmbeddedModules::LINT))
        }

        fn first_default_lint(module: &LonePreludeModule) -> Span {
            let notes = module.notes(LintDefaults::NOTE);
            let entry = notes.first().expect("prelude/lint states a default");
            entry.as_list().expect("a default is a list")[0].span()
        }

        fn gap_between_forms(module: &LonePreludeModule, inside: Span) -> Span {
            let text = module.sources.file(inside.source()).text();
            let gap = text
                .find(Self::FORM_GAP)
                .expect("the module holds two forms");
            Span::new(inside.source(), gap..gap + Self::FORM_GAP.len())
        }

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
    fn a_quoted_call_shows_the_written_call_at_its_location() {
        let lint = LonePreludeFixture::lint();
        let quoted = lint.quoted_call(LonePreludeFixture::first_default_lint(&lint));
        assert_eq!(
            quoted,
            "prelude/lint.knv:24:1: `(lint-default needless-type allow)`"
        );
    }

    #[test]
    fn a_location_is_the_start_of_the_written_call() {
        let lint = LonePreludeFixture::lint();
        let location = lint.location(LonePreludeFixture::first_default_lint(&lint));
        assert_eq!(location, "prelude/lint.knv:24:1");
    }

    #[test]
    fn a_quoted_site_outside_a_call_shows_the_site_itself() {
        let lint = LonePreludeFixture::lint();
        let inside = LonePreludeFixture::first_default_lint(&lint);
        let gap = LonePreludeFixture::gap_between_forms(&lint, inside);
        assert!(lint.quoted_call(gap).ends_with(": `\n\n`"));
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

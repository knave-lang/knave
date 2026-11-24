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

    pub(super) fn expanding(
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

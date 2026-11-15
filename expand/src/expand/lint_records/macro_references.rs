use rustc_hash::FxHashSet;

use crate::core::{
    BindingArena, BindingId, BindingKind, CoreModule, Expr, ExprKind, Lambda, Leaf, ModuleId,
    Namespace,
};
use crate::interpret::Closure;
use crate::modules::ModuleSources;
use crate::reader::{Form, FormKind};
use crate::scope::{BindingTable, Phase, Resolution};

pub(in crate::expand) struct MacroReferences<'t> {
    table: &'t BindingTable,
    bindings: &'t BindingArena,
    named: FxHashSet<BindingId>,
}

impl<'t> MacroReferences<'t> {
    pub(in crate::expand) fn new(table: &'t BindingTable, bindings: &'t BindingArena) -> Self {
        Self {
            table,
            bindings,
            named: FxHashSet::default(),
        }
    }

    pub(in crate::expand) fn named_by<'m>(
        mut self,
        macros: impl Iterator<Item = (BindingId, &'m Closure)>,
        modules: &[CoreModule],
    ) -> FxHashSet<BindingId> {
        let written: FxHashSet<ModuleId> = modules
            .iter()
            .filter(|module| ModuleSources::is_written(&module.path))
            .map(|module| module.id)
            .collect();
        for (binding, closure) in macros {
            if let BindingKind::Macro(module) = self.bindings.get(binding).kind()
                && written.contains(&module)
            {
                self.walk(&closure.lambda);
            }
        }
        self.named
    }

    fn walk(&mut self, macro_function: &Lambda) {
        let mut pending: Vec<&Expr> = macro_function.body.expressions().collect();
        while let Some(expr) = pending.pop() {
            match expr.kind() {
                ExprKind::Leaf(Leaf::Var(binding)) => self.note(*binding),
                ExprKind::Leaf(Leaf::Quote(form)) => self.note_template(form),
                _ => {}
            }
            pending.extend(expr.children());
        }
    }

    fn note_template(&mut self, form: &Form) {
        match form.kind() {
            FormKind::Symbol(symbol) => {
                if let Resolution::Bound(binding) =
                    self.table.resolve(symbol, Namespace::Value, Phase::Runtime)
                {
                    self.note(binding);
                }
            }
            FormKind::List(items) => items.iter().for_each(|item| self.note_template(item)),
            _ => {}
        }
    }

    fn note(&mut self, binding: BindingId) {
        if matches!(
            self.bindings.get(binding).kind(),
            BindingKind::Global(_) | BindingKind::CodeBuiltin(_)
        ) {
            self.named.insert(binding);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::core::{BindingId, ItemKind, ModulePath};
    use crate::expand::{Expander, Program};
    use crate::modules::ModuleSources;
    use crate::source::SourceMap;

    struct MacroReferencesFixture {
        program: Program,
    }

    impl MacroReferencesFixture {
        const MODULE: &str = "fixture";

        fn expand(text: &str) -> Self {
            let mut sources = SourceMap::default();
            let entry = sources.add(Self::MODULE, text);
            let module_sources = ModuleSources::new(PathBuf::new(), PathBuf::new());
            let program = Expander::new(&mut sources, module_sources)
                .expand(ModulePath::new(Self::MODULE), entry)
                .unwrap_or_else(|_| panic!("the program expands"));
            Self { program }
        }

        fn names(&self, name: &str) -> bool {
            self.program
                .compile_time_roots
                .is_named_by_a_macro(self.defined(name))
        }

        fn defined(&self, name: &str) -> BindingId {
            self.program
                .entry_module()
                .items
                .iter()
                .find_map(|item| match item.kind {
                    ItemKind::Def(binding, _)
                        if self.program.bindings.get(binding).name() == name =>
                    {
                        Some(binding)
                    }
                    _ => None,
                })
                .expect("the module defines the name")
        }
    }

    #[test]
    fn a_template_symbol_counts_even_when_the_macro_never_emits_it() {
        let fixture = MacroReferencesFixture::expand(
            "(defn helper () 1)\n(defmacro pick (x) (if false (template (helper)) x))",
        );
        assert!(fixture.names("helper"));
    }

    #[test]
    fn a_call_in_a_macro_body_counts_even_when_no_module_calls_the_macro() {
        let fixture = MacroReferencesFixture::expand(
            "(def size (fn (form) 1))\n(defmacro measured (x) (size x))",
        );
        assert!(fixture.names("size"));
    }

    #[test]
    fn a_definition_no_macro_names_is_not_named() {
        let fixture = MacroReferencesFixture::expand(
            "(defn helper () 1)\n(defn other () 2)\n(defmacro pick (x) (template (helper)))",
        );
        assert!(!fixture.names("other"));
    }
}

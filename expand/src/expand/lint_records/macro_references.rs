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
}

impl<'t> MacroReferences<'t> {
    pub(in crate::expand) fn new(table: &'t BindingTable, bindings: &'t BindingArena) -> Self {
        Self { table, bindings }
    }

    pub(in crate::expand) fn named_by<'m>(
        self,
        macros: impl Iterator<Item = (BindingId, &'m Closure)>,
        modules: &[CoreModule],
    ) -> FxHashSet<(ModuleId, BindingId)> {
        let written: FxHashSet<ModuleId> = modules
            .iter()
            .filter(|module| ModuleSources::is_written(&module.path))
            .map(|module| module.id)
            .collect();
        let mut named = FxHashSet::default();
        for (binding, closure) in macros {
            if let BindingKind::Macro(module) = self.bindings.get(binding).kind()
                && written.contains(&module)
            {
                self.walk(&closure.lambda, &mut |binding| {
                    named.insert((module, binding));
                });
            }
        }
        named
    }

    fn walk(&self, macro_function: &Lambda, named: &mut dyn FnMut(BindingId)) {
        let mut pending: Vec<&Expr> = macro_function.body.expressions().collect();
        while let Some(expr) = pending.pop() {
            match expr.kind() {
                ExprKind::Leaf(Leaf::Var(binding)) => self.note(*binding, named),
                ExprKind::Leaf(Leaf::Quote(form)) => self.note_template(form, named),
                _ => {}
            }
            pending.extend(expr.children());
        }
    }

    fn note_template(&self, form: &Form, named: &mut dyn FnMut(BindingId)) {
        match form.kind() {
            FormKind::Symbol(symbol) => {
                for namespace in Namespace::IMPORTED {
                    if let Resolution::Bound(binding) =
                        self.table.resolve_use(symbol, namespace, Phase::Runtime)
                    {
                        self.note(binding, named);
                    }
                }
            }
            FormKind::List(items) => items
                .iter()
                .for_each(|item| self.note_template(item, named)),
            _ => {}
        }
    }

    fn note(&self, binding: BindingId, named: &mut dyn FnMut(BindingId)) {
        if self.bindings.get(binding).kind().is_importable() {
            named(binding);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::BindingId;
    use crate::expand::Program;
    use crate::expand::expanded_module::ExpandedModule;

    struct MacroReferencesFixture {
        program: Program,
    }

    impl MacroReferencesFixture {
        fn expand(text: &str) -> Self {
            let program = ExpandedModule::of(text);
            Self { program }
        }

        fn names(&self, name: &str) -> bool {
            let defined = self.defined(name);
            self.program
                .compile_time_roots
                .named_by_macros_in(self.program.entry_module().id)
                .any(|named| named == defined)
        }

        fn defined(&self, name: &str) -> BindingId {
            let bindings = &self.program.bindings;
            let module = self.program.entry_module().id;
            bindings
                .ids()
                .find(|binding| {
                    let info = bindings.get(*binding);
                    info.name() == name && info.kind().defining_module() == Some(module)
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

    #[test]
    fn a_type_in_a_template_counts() {
        let fixture = MacroReferencesFixture::expand(
            "(defstruct point :x i64)\n(defmacro as-point (x) (template (the point x)))",
        );
        assert!(fixture.names("point"));
    }

    #[test]
    fn a_variant_in_a_template_counts() {
        let fixture = MacroReferencesFixture::expand(
            "(defenum shape (circle i64) dot)\n(defmacro round (x) (template (shape/circle x)))",
        );
        assert!(fixture.names("shape/circle"));
    }
}

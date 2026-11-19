use rustc_hash::{FxHashMap, FxHashSet};

use crate::core::{
    BindingArena, BindingId, BindingKind, CoreModule, Expr, ExprKind, Leaf, ModuleId, Namespace,
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
    ) -> FxHashSet<BindingId> {
        let written = modules
            .iter()
            .filter(|module| ModuleSources::is_written(&module.path))
            .map(|module| (module.id, module))
            .collect();
        let mut walk = NamedWalk {
            references: &self,
            written,
            named: FxHashSet::default(),
            followed: FxHashSet::default(),
        };
        for (binding, closure) in macros {
            if let BindingKind::Macro(module) = self.bindings.get(binding).kind()
                && walk.written.contains_key(&module)
            {
                walk.visit(closure.lambda.body.expressions().collect());
            }
        }
        walk.named
    }

    fn note_template(&self, form: &Form, named: &mut Vec<BindingId>) {
        match form.kind() {
            FormKind::Symbol(symbol) => {
                for namespace in Namespace::IMPORTED {
                    if let Resolution::Bound(binding) =
                        self.table.resolve_use(symbol, namespace, Phase::Runtime)
                    {
                        named.extend(self.note(binding));
                    }
                }
            }
            FormKind::List(items) => items
                .iter()
                .for_each(|item| self.note_template(item, named)),
            _ => {}
        }
    }

    fn note(&self, binding: BindingId) -> Option<BindingId> {
        self.bindings
            .get(binding)
            .kind()
            .is_importable()
            .then_some(binding)
    }
}

struct NamedWalk<'w, 't, 'm> {
    references: &'w MacroReferences<'t>,
    written: FxHashMap<ModuleId, &'m CoreModule>,
    named: FxHashSet<BindingId>,
    followed: FxHashSet<BindingId>,
}

impl<'m> NamedWalk<'_, '_, 'm> {
    fn visit<'e>(&mut self, mut pending: Vec<&'e Expr>)
    where
        'm: 'e,
    {
        while let Some(expr) = pending.pop() {
            let references = self.references;
            match expr.kind() {
                ExprKind::Leaf(Leaf::Var(binding)) => {
                    if let Some(helper) = references.note(*binding) {
                        pending.extend(self.body_to_follow(helper));
                    }
                }
                ExprKind::Leaf(Leaf::Quote(form)) => {
                    let mut quoted = Vec::new();
                    references.note_template(form, &mut quoted);
                    for helper in quoted {
                        pending.extend(self.body_to_follow(helper));
                    }
                }
                _ => {}
            }
            pending.extend(expr.children());
        }
    }

    fn body_to_follow(&mut self, helper: BindingId) -> Option<&'m Expr> {
        self.named.insert(helper);
        if !self.followed.insert(helper) {
            return None;
        }
        self.body_of(helper)
    }

    fn body_of(&self, binding: BindingId) -> Option<&'m Expr> {
        let BindingKind::Global(module) = self.references.bindings.get(binding).kind() else {
            return None;
        };
        let defining = self.written.get(&module)?;
        let body = defining
            .definitions()
            .find(|(defined, _)| *defined == binding)?;
        Some(&**body.1)
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
            self.program.compile_time_roots.contains(defined)
        }

        fn names_in(&self, name: &str, path: &str) -> bool {
            let module = self
                .program
                .modules
                .iter()
                .find(|module| module.path.as_str() == path)
                .expect("the program loads the module")
                .id;
            let bindings = &self.program.bindings;
            let defined = bindings
                .ids()
                .find(|binding| {
                    let info = bindings.get(*binding);
                    info.name() == name && info.kind().defining_module() == Some(module)
                })
                .expect("the module defines the name");
            self.program.compile_time_roots.contains(defined)
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
    fn a_quote_in_a_helper_the_macro_calls_counts() {
        let fixture = MacroReferencesFixture::expand(
            "(defn mark () 1)\n(def helper (fn (form) (quote mark)))\n\
             (defmacro marked (x) (helper x))",
        );
        assert!(fixture.names("mark"));
    }

    #[test]
    fn a_helper_a_template_names_before_the_macro_calls_it_is_still_followed() {
        let fixture = MacroReferencesFixture::expand(
            "(defn mark () 1)\n(def helper (fn (form) (quote mark)))\n\
             (defmacro marked (x) (if true (helper x) (template (helper))))",
        );
        assert!(fixture.names("mark"));
    }

    #[test]
    fn a_quote_in_a_function_a_template_names_counts() {
        let fixture = MacroReferencesFixture::expand(
            "(defn mark () 1)\n(defn runtime () (quote mark))\n\
             (defmacro run (x) (template (runtime)))",
        );
        assert!(fixture.names("mark"));
    }

    #[test]
    fn a_prelude_macro_names_nothing() {
        let fixture = MacroReferencesFixture::expand(
            "(use prelude/test run-tests)\n(defn main () (println 1))",
        );
        assert!(!fixture.names_in("start-run", "prelude/test"));
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

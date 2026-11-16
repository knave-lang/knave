use std::rc::Rc;

use crate::core::{BindingArena, BindingId, BindingKind, ModuleId, Namespace, VariantInfo};
use crate::reader::Symbol;
use crate::scope::{BindingTable, LargestCandidates, Phase, QualifiedEntries, Resolution};

#[derive(Clone, Copy)]
pub(super) struct Bindings<'t> {
    table: &'t BindingTable,
    bindings: &'t BindingArena,
    phase: Phase,
}

impl<'t> Bindings<'t> {
    pub(super) fn new(table: &'t BindingTable, bindings: &'t BindingArena, phase: Phase) -> Self {
        Self {
            table,
            bindings,
            phase,
        }
    }

    pub(super) fn resolve(&self, symbol: &Symbol, namespace: Namespace) -> Option<BindingId> {
        self.lookup(symbol, namespace).bound()
    }

    #[inline(always)]
    pub(super) fn lookup(&self, symbol: &Symbol, namespace: Namespace) -> LargestCandidates<'t> {
        self.table
            .visible_names()
            .largest_candidates(symbol, namespace, self.phase)
    }

    pub(super) fn resolve_use(&self, symbol: &Symbol, namespace: Namespace) -> Option<BindingId> {
        Self::bound(self.table.resolve_use(symbol, namespace, self.phase))
    }

    pub(super) fn resolve_variants_ending_in(
        &self,
        symbol: &Symbol,
        accepts: impl Fn(VariantInfo) -> bool,
    ) -> Vec<BindingId> {
        let variants = self.variant_entries(symbol, accepts);
        variants.mark_uses();
        variants.bindings()
    }

    fn variant_entries(
        &self,
        symbol: &Symbol,
        accepts: impl Fn(VariantInfo) -> bool,
    ) -> QualifiedEntries<'t> {
        let mut variants = self.table.visible_names().qualified_entries_ending_in(
            symbol,
            Namespace::Value,
            self.phase,
        );
        variants.retain(|binding| {
            matches!(self.bindings.get(binding).kind(), BindingKind::Variant(_, info) if accepts(info))
        });
        variants
    }

    fn bound(resolution: Resolution) -> Option<BindingId> {
        match resolution {
            Resolution::Bound(binding) => Some(binding),
            Resolution::Unbound | Resolution::Ambiguous(_) => None,
        }
    }

    pub(super) fn definition_module(&self, binding: BindingId) -> Option<ModuleId> {
        self.bindings.get(binding).kind().defining_module()
    }

    pub(super) fn visible_named_like(
        &self,
        symbol: &Symbol,
        namespace: Namespace,
    ) -> Vec<(Rc<str>, BindingId)> {
        self.table
            .visible_names()
            .name_bindings(symbol, namespace, self.phase)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::core::{ModuleId, ModulePath, Namespace};
    use crate::expand::Expander;
    use crate::modules::ModuleSources;
    use crate::reader::{Form, FormKind, Symbol};
    use crate::source::{SourceId, SourceMap, Span};

    struct BindingsFixture {
        sources: SourceMap,
        entry: SourceId,
    }

    impl BindingsFixture {
        const MODULE: &str = "names";
        const PROGRAM: &str = "(defn twice (x) (* 2 x))\n";
        const DEFINED: &str = "twice";
        const UNBOUND: &str = "thrice";
        const MISSPELLED: &str = "twise";

        fn new() -> Self {
            let mut sources = SourceMap::default();
            let entry = sources.add(Self::MODULE, Self::PROGRAM);
            Self { sources, entry }
        }

        fn with_bindings<T>(
            &mut self,
            name: &str,
            read: impl FnOnce(&super::Bindings<'_>, &Symbol, ModuleId) -> T,
        ) -> T {
            let entry = self.entry;
            let mut expander = Expander::new(
                &mut self.sources,
                ModuleSources::new(PathBuf::new(), PathBuf::new()),
            );
            let module = expander.load_source_module(ModulePath::new(Self::MODULE), &[entry]);
            let mut form = Form::new(FormKind::Symbol(Symbol::new(name)), Span::new(entry, 0..0));
            let scopes = expander.modules.get(module).scopes();
            form.add_scope(scopes.base());
            form.add_scope(scopes.body());
            let symbol = form.as_symbol().expect("the form is a symbol").clone();
            read(&expander.bindings(), &symbol, module)
        }
    }

    #[test]
    fn a_name_the_module_defines_resolves_to_a_binding_of_the_module() {
        let mut fixture = BindingsFixture::new();
        let defining =
            fixture.with_bindings(BindingsFixture::DEFINED, |bindings, symbol, module| {
                let binding = bindings.resolve(symbol, Namespace::Value);
                (
                    binding.and_then(|binding| bindings.definition_module(binding)),
                    module,
                )
            });
        assert_eq!(defining.0, Some(defining.1));
    }

    #[test]
    fn an_unbound_name_resolves_to_nothing() {
        let mut fixture = BindingsFixture::new();
        let binding = fixture.with_bindings(BindingsFixture::UNBOUND, |bindings, symbol, _| {
            bindings.resolve(symbol, Namespace::Value)
        });
        assert_eq!(binding, None);
    }

    #[test]
    fn the_names_visible_from_a_symbol_include_the_module_definitions() {
        let mut fixture = BindingsFixture::new();
        let names = fixture.with_bindings(BindingsFixture::MISSPELLED, |bindings, symbol, _| {
            bindings
                .visible_named_like(symbol, Namespace::Value)
                .into_iter()
                .map(|(name, _)| name.to_string())
                .collect::<Vec<_>>()
        });
        assert!(
            names.iter().any(|name| name == BindingsFixture::DEFINED),
            "{names:?}"
        );
    }
}

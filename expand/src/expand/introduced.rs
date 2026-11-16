use super::Expander;
use crate::core::{BinderOrigin, BindingId, BindingKind, ModuleId, Namespace};
use crate::reader::Form;

impl Expander<'_> {
    pub(super) fn note_global(&mut self, binding: BindingId, binder: &Form) {
        if self.tables.scopes.introductions().carried_by_binder(binder) {
            self.tables
                .bindings
                .set_origin(binding, BinderOrigin::Introduced);
        }
        if matches!(self.kind(binding), BindingKind::Macro(_)) {
            self.tables.global_macros.push((binding, binder.clone()));
        }
    }

    pub(super) fn record_answering_macros(&mut self, module: ModuleId) {
        let (macros, others): (Vec<_>, Vec<_>) = std::mem::take(&mut self.tables.global_macros)
            .into_iter()
            .partition(|(binding, _)| self.kind(*binding) == BindingKind::Macro(module));
        self.tables.global_macros = others;
        for (answering, name) in macros {
            if let Some(function) = self.answered_function(answering, name) {
                self.tables
                    .bindings
                    .record_answering_macro(function, answering);
            }
        }
    }

    pub(super) fn answered_global(
        &mut self,
        module: ModuleId,
        binding: BindingId,
        name: &Form,
    ) -> Option<BindingId> {
        if self.kind(binding) != BindingKind::Macro(module) {
            return None;
        }
        self.answered_function(binding, name.clone())
    }

    fn answered_function(&mut self, answering: BindingId, name: Form) -> Option<BindingId> {
        let expansion = self.discarded_expansion(answering, name).ok()?;
        let symbol = expansion.as_symbol()?;
        let lookup = self.bindings().lookup(symbol, Namespace::Value);
        lookup.bound_where(|function| self.tables.bindings.origin(function).is_introduced())
    }
}

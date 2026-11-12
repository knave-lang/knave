use super::binding_symbols::BindingSymbols;
use super::bound_names::BoundNames;
use super::core_forms::CoreForms;
use super::read_back_definition::ReadBackDefinition;
use crate::core::{BindingArena, CoreModule};
use crate::scope::ScopeSupply;

pub struct ReadBack<'p> {
    bindings: &'p BindingArena,
    symbols: BindingSymbols<'p>,
}

impl<'p> ReadBack<'p> {
    pub fn new(bindings: &'p BindingArena, supply: &'p mut ScopeSupply) -> Self {
        Self {
            bindings,
            symbols: BindingSymbols::new(supply),
        }
    }

    pub fn definitions<'m>(
        &mut self,
        modules: impl Iterator<Item = &'m CoreModule>,
    ) -> Vec<ReadBackDefinition> {
        let mut forms = CoreForms::new(self.bindings, &mut self.symbols);
        modules
            .flat_map(|module| module.definitions())
            .map(|(binding, value)| forms.definition(binding, value))
            .collect()
    }

    pub fn into_names(self) -> BoundNames {
        self.symbols.into_names()
    }
}

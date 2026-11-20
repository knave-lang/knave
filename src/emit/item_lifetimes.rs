use super::fresh_lifetimes::FreshLifetimes;
use super::types::RustTypes;
use crate::core::BindingId;
use crate::types::{Type, TypedProgram};

pub(super) struct ItemLifetimes<'a> {
    program: &'a TypedProgram,
}

impl<'a> ItemLifetimes<'a> {
    pub(super) fn new(program: &'a TypedProgram) -> Self {
        Self { program }
    }

    pub(super) fn of_implementor(&self, implementor: &Type) -> Option<String> {
        let Type::Named(binding, _) = implementor else {
            return None;
        };
        if !self.program.has_lifetime_parameter(*binding) {
            return None;
        }
        Some(self.written_lifetime(*binding).map_or_else(
            || self.unused_name(&[]),
            |lifetime| RustTypes::binding_lifetime(self.program, lifetime),
        ))
    }

    pub(super) fn unused_name(&self, lifetimes: &[BindingId]) -> String {
        let used: Vec<String> = lifetimes
            .iter()
            .map(|binding| RustTypes::binding_lifetime(self.program, *binding))
            .collect();
        FreshLifetimes::unused_name(&used)
    }

    fn written_lifetime(&self, binding: BindingId) -> Option<BindingId> {
        let types = self.program.declarations.types();
        match types.structs().get(binding) {
            Some(definition) => definition.lifetimes.first().copied(),
            None => (types.enums().get(binding))
                .and_then(|definition| definition.lifetimes.first().copied()),
        }
    }
}

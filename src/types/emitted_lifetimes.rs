use std::iter;

use super::ty::{Lifetime, Type};
use super::typed::TypedProgram;
use crate::core::BindingId;

pub struct EmittedLifetimes<'p> {
    program: &'p TypedProgram,
}

impl<'p> EmittedLifetimes<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub fn of(&self, ty: &Type) -> Vec<Lifetime> {
        match ty {
            Type::Function(..) => Vec::new(),
            Type::Ref(lifetime, referent) | Type::Mut(lifetime, referent) => {
                iter::once(*lifetime).chain(self.of(referent)).collect()
            }
            Type::Named(binding, arguments) => self
                .inferred_lifetime_of(*binding)
                .into_iter()
                .chain(arguments.iter().flat_map(|argument| self.of(argument)))
                .collect(),
            other => other
                .parts()
                .into_iter()
                .flat_map(|part| self.of(part))
                .collect(),
        }
    }

    fn inferred_lifetime_of(&self, binding: BindingId) -> Option<Lifetime> {
        self.program
            .has_lifetime_parameter(binding)
            .then_some(Lifetime::Elided)
    }
}

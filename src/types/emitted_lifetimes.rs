use super::named_lifetimes::NamedLifetimes;
use super::ty::Lifetime;
use super::typed::TypedProgram;
use crate::core::BindingId;

pub struct EmittedLifetimes<'p> {
    program: &'p TypedProgram,
}

impl<'p> EmittedLifetimes<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }
}

impl NamedLifetimes for EmittedLifetimes<'_> {
    fn of_named(&self, binding: BindingId) -> Option<Lifetime> {
        self.program
            .has_lifetime_parameter(binding)
            .then_some(Lifetime::Elided)
    }
}

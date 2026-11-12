use std::rc::Rc;

use crate::core::{BindingId, RustTemplate};
use crate::types::{Bound, GenericId, Type, TypedProgram};

pub struct ClauseChoice<'p> {
    program: &'p TypedProgram,
}

impl<'p> ClauseChoice<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub fn applied(
        &self,
        template: &Rc<RustTemplate>,
        binding: BindingId,
        parameter: &Type,
    ) -> Rc<RustTemplate> {
        let implements = |bound: &Bound| self.satisfies(bound, parameter);
        match self
            .program
            .template_clauses
            .chosen(binding, parameter, &implements)
        {
            Some(clause) => Rc::new(RustTemplate {
                template: Rc::clone(clause),
                ..RustTemplate::clone(template)
            }),
            None => Rc::clone(template),
        }
    }

    fn satisfies(&self, bound: &Bound, parameter: &Type) -> bool {
        let traits = self.program.declarations.traits().table();
        traits.satisfies(bound, parameter, &|generic, required| {
            self.declares(generic, required)
        })
    }

    fn declares(&self, generic: GenericId, required: &Bound) -> bool {
        let traits = self.program.declarations.traits().table();
        self.program.generics.get(&generic).is_some_and(|param| {
            param
                .bounds
                .iter()
                .any(|declared| traits.implies(declared, required, generic))
        })
    }
}

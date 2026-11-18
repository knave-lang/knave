use std::rc::Rc;

use super::program_definitions::ProgramDefinitions;
use super::scheme_bindings::SchemeBindings;
use super::ty::{Bound, GenericId, Type};
use super::typed::{TypedDefinition, TypedExpr, TypedExprKind, TypedLeaf, TypedProgram};
use crate::core::{ParsedTemplate, RustTemplate};

pub struct ClauseChoice<'p> {
    program: &'p TypedProgram,
}

impl<'p> ClauseChoice<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub fn for_call(
        &self,
        template: &Rc<RustTemplate>,
        definition: &TypedDefinition,
        instantiated: &Type,
    ) -> Rc<RustTemplate> {
        match self.selected_clause(definition, instantiated) {
            Some(clause) => Rc::new(RustTemplate {
                template: Rc::clone(clause),
                ..RustTemplate::clone(template)
            }),
            None => Rc::clone(template),
        }
    }

    pub fn read_selected_template<T>(
        &self,
        callee: &TypedExpr,
        absent: T,
        read: impl Fn(&ParsedTemplate) -> T,
    ) -> T {
        match self
            .callee_clause(callee)
            .and_then(|clause| ParsedTemplate::parse(clause).ok())
        {
            Some(parsed) => read(&parsed),
            None => self
                .program
                .template_borrows
                .read_template(callee, absent, read),
        }
    }

    fn callee_clause(&self, callee: &TypedExpr) -> Option<&'p Rc<str>> {
        let TypedExprKind::Leaf(TypedLeaf::Global(binding)) = &callee.kind else {
            return None;
        };
        if !self.program.template_clauses.lists(*binding) {
            return None;
        }
        let definition = ProgramDefinitions::new(self.program).find(*binding)?;
        self.selected_clause(definition, &callee.ty)
    }

    fn selected_clause(
        &self,
        definition: &TypedDefinition,
        instantiated: &Type,
    ) -> Option<&'p Rc<str>> {
        let chooser = definition.scheme.clause_chooser()?;
        let binder = SchemeBindings::new(definition).of_parameter(instantiated, chooser.id)?;
        let parameter = self.decided(binder);
        let implements = |bound: &Bound| self.satisfies(bound, &parameter);
        self.program
            .template_clauses
            .chosen(definition.binding, &parameter, &implements)
    }

    fn decided(&self, binder: Type) -> Type {
        let literal = match &binder {
            Type::Generic(generic) => self.program.literal_generics.get(generic),
            _ => None,
        };
        match literal {
            Some(literal) => Type::Primitive(literal.default_type()),
            None => binder,
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

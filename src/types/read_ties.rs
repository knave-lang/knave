use super::field_reader::FieldReader;
use super::field_traits::FieldTraitPlan;
use super::mentioned_generics::MentionedGenerics;
use super::ty::{Bound, GenericId, Scheme, Type};
use super::typed::TypedProgram;

pub(super) struct ReadTies<'p> {
    program: &'p TypedProgram,
    scheme: &'p Scheme,
}

impl<'p> ReadTies<'p> {
    pub(super) fn new(program: &'p TypedProgram, scheme: &'p Scheme) -> Self {
        Self { program, scheme }
    }

    pub(super) fn hold(&self, reader: &FieldReader, plan: &FieldTraitPlan) -> bool {
        let reads = self.read_variables();
        let mut others = (self.scheme.params.iter())
            .flat_map(|param| param.bounds.iter().map(move |bound| (param.id, bound)))
            .filter(|(_, bound)| !self.is_field_bound(bound));
        others.any(|(bounded, bound)| {
            let named = (bound.argument.as_ref())
                .map(|argument| MentionedGenerics::of(argument).into_ids())
                .unwrap_or_default();
            let touches = reads.contains(&bounded) || named.iter().any(|id| reads.contains(id));
            let ties_other = named.iter().any(|id| !reads.contains(id));
            let on_own_read = reader.read == Type::Generic(bounded) && named.is_empty();
            touches && (ties_other || (on_own_read && !self.satisfied(bound, &plan.read)))
        })
    }

    fn read_variables(&self) -> Vec<GenericId> {
        let bounds = (self.scheme.params.iter()).flat_map(|param| &param.bounds);
        let field_bounds = bounds.filter(|bound| self.is_field_bound(bound));
        (field_bounds.filter_map(|bound| bound.argument.as_ref()))
            .flat_map(|read| MentionedGenerics::of(read).into_ids())
            .collect()
    }

    fn is_field_bound(&self, bound: &Bound) -> bool {
        let field_traits = self
            .program
            .declarations
            .traits()
            .rust_paths()
            .field_traits();
        field_traits.bindings().is_field_bound(bound)
    }

    fn satisfied(&self, bound: &Bound, read: &Type) -> bool {
        let traits = self.program.declarations.traits().table();
        traits.satisfies(bound, read, &|_, _| false)
    }
}

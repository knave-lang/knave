use super::field_reader::FieldReader;
use super::field_traits::FieldTraitPlan;
use super::mentioned_generics::MentionedGenerics;
use super::plain_bound::PlainBound;
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
        let plain = PlainBound::new(self.program.declarations.traits().table());
        let mut others = (self.scheme.params.iter())
            .flat_map(|param| {
                param
                    .bounds
                    .iter()
                    .map(move |bound| ScopedBound::new(param.id, bound))
            })
            .filter(|scoped| !self.is_field_bound(scoped.bound));
        others.any(|scoped| {
            let unmet_on_read =
                scoped.is_plain_on(&reader.read) && !plain.is_met_by(scoped.bound, &plan.read);
            scoped.touches(&reads) && (scoped.names_other_than(&reads) || unmet_on_read)
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
}

struct ScopedBound<'b> {
    bounded: GenericId,
    bound: &'b Bound,
    named: Vec<GenericId>,
}

impl<'b> ScopedBound<'b> {
    fn new(bounded: GenericId, bound: &'b Bound) -> Self {
        let named = (bound.argument.as_ref())
            .map(|argument| MentionedGenerics::of(argument).into_ids())
            .unwrap_or_default();
        Self {
            bounded,
            bound,
            named,
        }
    }

    fn touches(&self, reads: &[GenericId]) -> bool {
        reads.contains(&self.bounded) || self.named.iter().any(|id| reads.contains(id))
    }

    fn names_other_than(&self, reads: &[GenericId]) -> bool {
        self.named.iter().any(|id| !reads.contains(id))
    }

    fn is_plain_on(&self, read: &Type) -> bool {
        *read == Type::Generic(self.bounded) && self.named.is_empty()
    }
}

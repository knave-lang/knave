use super::admissible_types::AdmissibleTypes;
use super::bound_proof::BoundProof;
use super::generic_bound_tables::GenericBoundTables;
use super::implied_bounds::ImpliedBounds;
use super::ty::{Bound, GenericId, Type};
use super::unifier::Unifier;
use crate::source::Span;

pub(super) struct GenericBoundCheck<'a> {
    unifier: &'a mut Unifier,
    implied_bounds: &'a mut ImpliedBounds,
    tables: GenericBoundTables<'a>,
}

impl<'a> GenericBoundCheck<'a> {
    pub(super) fn new(
        unifier: &'a mut Unifier,
        implied_bounds: &'a mut ImpliedBounds,
        tables: GenericBoundTables<'a>,
    ) -> Self {
        Self {
            unifier,
            implied_bounds,
            tables,
        }
    }

    pub(super) fn settle(&mut self, generic: GenericId, bound: Bound, span: Span) -> BoundProof {
        if self.declares_bound(generic, &bound) {
            return BoundProof::Proven;
        }
        if self.implied_bounds.is_impl_generic(generic) {
            self.implied_bounds.record_from_body(generic, bound, span);
            return BoundProof::Proven;
        }
        if self.apply_declared_bound(generic, &bound, span) {
            return BoundProof::Proven;
        }
        if self.implies(generic, &bound) {
            if self.declared_bound_implies(generic, &bound) {
                return BoundProof::Proven;
            }
            self.implied_bounds.record(generic, bound);
            return BoundProof::Proven;
        }
        BoundProof::Unproven(bound)
    }

    fn declares_bound(&self, generic: GenericId, bound: &Bound) -> bool {
        self.unifier
            .generic_bounds(generic)
            .iter()
            .any(|declared| declared.meets(bound, generic))
    }

    fn apply_declared_bound(&mut self, generic: GenericId, bound: &Bound, span: Span) -> bool {
        let Some(required) = &bound.argument else {
            return false;
        };
        let declared: Vec<Type> = self
            .unifier
            .generic_bounds(generic)
            .iter()
            .filter(|declared| declared.binding == bound.binding)
            .filter_map(|declared| declared.argument.clone())
            .collect();
        declared
            .into_iter()
            .any(|declared| self.unifier.unify(&declared, required, span).is_ok())
    }

    fn implies(&self, generic: GenericId, bound: &Bound) -> bool {
        if self.declared_bound_implies(generic, bound) {
            return true;
        }
        if bound.argument.is_some() {
            return false;
        }
        self.admissible_types().all_satisfy(generic, bound)
    }

    fn declared_bound_implies(&self, generic: GenericId, bound: &Bound) -> bool {
        self.unifier
            .generic_bounds(generic)
            .iter()
            .any(|declared| self.tables.traits.implies(declared, bound, generic))
    }

    fn admissible_types(&self) -> AdmissibleTypes<'a> {
        AdmissibleTypes::new(self.tables.generics, self.tables.traits)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Bound;
    use crate::types::bound_proof::BoundProof;
    use crate::types::generic_bound_fixture::GenericBoundFixture;

    #[test]
    fn a_declared_bound_is_proven_without_recording_it() {
        let mut fixture = GenericBoundFixture::new();
        fixture.declare(vec![Bound::plain(GenericBoundFixture::EQUAL)]);
        let proof = fixture.settle(Bound::plain(GenericBoundFixture::EQUAL));
        assert!(matches!(proof, BoundProof::Proven));
        assert!(fixture.implied_traits().is_empty());
    }

    #[test]
    fn an_impl_generic_records_the_bound_its_body_needs() {
        let mut fixture = GenericBoundFixture::new();
        fixture.declare(Vec::new());
        fixture.mark_impl_generic();
        fixture.settle(Bound::plain(GenericBoundFixture::EQUAL));
        assert_eq!(fixture.implied_traits(), [GenericBoundFixture::EQUAL]);
    }

    #[test]
    fn a_declared_argument_becomes_the_required_argument() {
        let mut fixture = GenericBoundFixture::new();
        let int = GenericBoundFixture::int();
        fixture.declare(vec![Bound::with_argument(
            GenericBoundFixture::EQUAL,
            int.clone(),
        )]);
        let required = fixture.var();
        fixture.settle(Bound::with_argument(
            GenericBoundFixture::EQUAL,
            required.clone(),
        ));
        assert_eq!(fixture.resolve(&required), int);
    }

    #[test]
    fn a_supertrait_of_a_declared_bound_is_proven_without_recording_it() {
        let mut fixture = GenericBoundFixture::new();
        fixture.declare(vec![Bound::plain(GenericBoundFixture::ORDERED)]);
        let proof = fixture.settle(Bound::plain(GenericBoundFixture::EQUAL));
        assert!(matches!(proof, BoundProof::Proven));
        assert!(fixture.implied_traits().is_empty());
    }

    #[test]
    fn a_bound_every_admissible_type_meets_is_recorded_as_implied() {
        let mut fixture = GenericBoundFixture::new();
        fixture.declare(vec![Bound::plain(GenericBoundFixture::SHOWN)]);
        fixture.settle(Bound::plain(GenericBoundFixture::EQUAL));
        assert_eq!(fixture.implied_traits(), [GenericBoundFixture::EQUAL]);
    }

    #[test]
    fn a_bound_nothing_proves_is_unproven() {
        let mut fixture = GenericBoundFixture::new();
        fixture.declare(Vec::new());
        let proof = fixture.settle(Bound::plain(GenericBoundFixture::EQUAL));
        assert!(matches!(proof, BoundProof::Unproven(_)));
    }

    #[test]
    fn an_impl_generic_records_no_bound_it_declares() {
        let mut fixture = GenericBoundFixture::new();
        fixture.declare(vec![Bound::plain(GenericBoundFixture::EQUAL)]);
        fixture.mark_impl_generic();
        fixture.settle(Bound::plain(GenericBoundFixture::EQUAL));
        assert!(fixture.implied_traits().is_empty());
    }

    #[test]
    fn a_declared_argument_of_another_trait_leaves_the_required_argument_open() {
        let mut fixture = GenericBoundFixture::new();
        fixture.declare(vec![Bound::with_argument(
            GenericBoundFixture::SHOWN,
            GenericBoundFixture::int(),
        )]);
        let required = fixture.var();
        fixture.settle(Bound::with_argument(
            GenericBoundFixture::EQUAL,
            required.clone(),
        ));
        assert_eq!(fixture.resolve(&required), required);
    }

    #[test]
    fn a_bound_with_an_argument_is_not_implied_by_the_admissible_types() {
        let mut fixture = GenericBoundFixture::new();
        fixture.declare(vec![Bound::plain(GenericBoundFixture::SHOWN)]);
        let proof = fixture.settle(Bound::with_argument(
            GenericBoundFixture::PAIRED,
            GenericBoundFixture::int(),
        ));
        assert!(matches!(proof, BoundProof::Unproven(_)));
    }
}

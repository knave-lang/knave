use rustc_hash::{FxHashMap, FxHashSet};

use super::ty::{Bound, GenericId, GenericParam};

#[derive(Default)]
pub(super) struct ImpliedBounds {
    impl_generics: FxHashSet<GenericId>,
    bounds: FxHashMap<GenericId, Vec<Bound>>,
}

impl ImpliedBounds {
    pub(super) fn mark_impl_generics(&mut self, params: &[GenericParam]) {
        self.impl_generics
            .extend(params.iter().map(|param| param.id));
    }

    pub(super) fn is_impl_generic(&self, generic: GenericId) -> bool {
        self.impl_generics.contains(&generic)
    }

    pub(super) fn record(&mut self, generic: GenericId, bound: Bound) {
        let implied = self.bounds.entry(generic).or_default();
        if !implied.contains(&bound) {
            implied.push(bound);
        }
    }

    pub(super) fn of(&self, generic: GenericId) -> impl Iterator<Item = &Bound> {
        self.bounds.get(&generic).into_iter().flatten()
    }

    pub(super) fn into_bounds(self) -> FxHashMap<GenericId, Vec<Bound>> {
        self.bounds
    }
}

#[cfg(test)]
mod tests {
    use super::ImpliedBounds;
    use crate::core::BindingId;
    use crate::types::ty::{Bound, GenericId, GenericParam};

    struct ImpliedBoundsFixture {
        bounds: ImpliedBounds,
    }

    impl ImpliedBoundsFixture {
        const DISPLAY: BindingId = BindingId::from_index(0);
        const DEBUG: BindingId = BindingId::from_index(1);

        fn generic() -> GenericId {
            GenericId::new(0)
        }

        fn other() -> GenericId {
            GenericId::new(1)
        }

        fn new() -> Self {
            Self {
                bounds: ImpliedBounds::default(),
            }
        }

        fn recording(mut self, binding: BindingId) -> Self {
            self.bounds.record(Self::generic(), Bound::plain(binding));
            self
        }

        fn bounds_of(&self, generic: GenericId) -> Vec<Bound> {
            self.bounds.of(generic).cloned().collect()
        }

        fn param(id: GenericId) -> GenericParam {
            GenericParam {
                id,
                name: "a".into(),
                bounds: Vec::new(),
            }
        }
    }

    #[test]
    fn a_bound_recorded_twice_is_kept_once() {
        let fixture = ImpliedBoundsFixture::new()
            .recording(ImpliedBoundsFixture::DISPLAY)
            .recording(ImpliedBoundsFixture::DISPLAY);
        assert_eq!(
            fixture.bounds_of(ImpliedBoundsFixture::generic()),
            vec![Bound::plain(ImpliedBoundsFixture::DISPLAY)]
        );
    }

    #[test]
    fn two_different_bounds_are_both_kept() {
        let fixture = ImpliedBoundsFixture::new()
            .recording(ImpliedBoundsFixture::DISPLAY)
            .recording(ImpliedBoundsFixture::DEBUG);
        assert_eq!(fixture.bounds_of(ImpliedBoundsFixture::generic()).len(), 2);
    }

    #[test]
    fn a_generic_without_recorded_bounds_implies_none() {
        let fixture = ImpliedBoundsFixture::new().recording(ImpliedBoundsFixture::DISPLAY);
        assert!(fixture.bounds_of(ImpliedBoundsFixture::other()).is_empty());
    }

    #[test]
    fn the_recorded_bounds_leave_with_their_generic() {
        let fixture = ImpliedBoundsFixture::new().recording(ImpliedBoundsFixture::DISPLAY);
        let bounds = fixture.bounds.into_bounds();
        assert_eq!(bounds[&ImpliedBoundsFixture::generic()].len(), 1);
    }

    #[test]
    fn a_generic_marked_from_a_signature_is_an_impl_generic() {
        let mut fixture = ImpliedBoundsFixture::new();
        let param = ImpliedBoundsFixture::param(ImpliedBoundsFixture::generic());
        fixture.bounds.mark_impl_generics(&[param]);
        assert!(
            fixture
                .bounds
                .is_impl_generic(ImpliedBoundsFixture::generic())
        );
    }

    #[test]
    fn an_unmarked_generic_is_no_impl_generic() {
        let mut fixture = ImpliedBoundsFixture::new();
        let param = ImpliedBoundsFixture::param(ImpliedBoundsFixture::generic());
        fixture.bounds.mark_impl_generics(&[param]);
        assert!(
            !fixture
                .bounds
                .is_impl_generic(ImpliedBoundsFixture::other())
        );
    }
}

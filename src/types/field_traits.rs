use std::collections::BTreeMap;

use super::field_trait_bindings::FieldTraitBindings;
use super::read_kind::ReadKind;
use super::ty::Type;
use crate::core::BindingId;

#[derive(Default)]
pub struct FieldTraits {
    bindings: FieldTraitBindings,
    plans: BTreeMap<BindingId, FieldTraitPlan>,
}

#[derive(Clone)]
pub struct FieldTraitPlan {
    pub read: Type,
    pub kind: ReadKind,
    pub implementors: Vec<Type>,
}

impl FieldTraits {
    pub const MODULE: &'static str = "field_traits";

    pub fn bindings(&self) -> &FieldTraitBindings {
        &self.bindings
    }

    pub(super) fn bindings_mut(&mut self) -> &mut FieldTraitBindings {
        &mut self.bindings
    }

    pub(super) fn settle(&mut self, binding: BindingId, plan: FieldTraitPlan) {
        self.plans.insert(binding, plan);
    }

    pub fn plans(&self) -> impl Iterator<Item = (BindingId, &FieldTraitPlan)> {
        self.plans.iter().map(|(binding, plan)| (*binding, plan))
    }
}

#[cfg(test)]
mod tests {
    use super::FieldTraits;
    use crate::core::BindingId;

    const HAS_NAME: BindingId = BindingId::from_index(0);

    struct FieldTraitsFixture {
        field_traits: FieldTraits,
    }

    impl FieldTraitsFixture {
        fn new() -> Self {
            let mut field_traits = FieldTraits::default();
            field_traits.bindings_mut().record("name".into(), HAS_NAME);
            Self { field_traits }
        }
    }

    #[test]
    fn finds_a_field_trait_by_field_and_by_binding() {
        let fixture = FieldTraitsFixture::new();
        assert_eq!(
            fixture.field_traits.bindings().trait_of("name"),
            Some(HAS_NAME)
        );
        assert_eq!(fixture.field_traits.bindings().trait_of("age"), None);
        assert_eq!(
            fixture
                .field_traits
                .bindings()
                .field_of(HAS_NAME)
                .map(|field| &**field),
            Some("name")
        );
    }
}

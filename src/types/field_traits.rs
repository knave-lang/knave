use std::collections::BTreeMap;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::mentioned_generics::MentionedGenerics;
use super::read_kind::ReadKind;
use super::ty::{Bound, GenericId, GenericParam, Type, TypeVar};
use super::unifier::Unifier;
use crate::core::BindingId;

#[derive(Default)]
pub struct FieldTraits {
    by_field: FxHashMap<Rc<str>, BindingId>,
    by_binding: FxHashMap<BindingId, Rc<str>>,
    plans: BTreeMap<BindingId, FieldTraitPlan>,
}

#[derive(Clone)]
pub struct FieldTraitPlan {
    pub read: Type,
    pub kind: ReadKind,
    pub implementors: Vec<Type>,
}

impl FieldTraits {
    pub fn record(&mut self, field: Rc<str>, binding: BindingId) {
        self.by_binding.insert(binding, Rc::clone(&field));
        self.by_field.insert(field, binding);
    }

    pub fn trait_of(&self, field: &str) -> Option<BindingId> {
        self.by_field.get(field).copied()
    }

    pub fn field_of(&self, binding: BindingId) -> Option<&Rc<str>> {
        self.by_binding.get(&binding)
    }

    pub(super) fn settle(&mut self, binding: BindingId, plan: FieldTraitPlan) {
        self.plans.insert(binding, plan);
    }

    pub fn plans(&self) -> impl Iterator<Item = (BindingId, &FieldTraitPlan)> {
        self.plans.iter().map(|(binding, plan)| (*binding, plan))
    }

    pub(super) fn add_read_vars(
        &self,
        vars: &mut Vec<TypeVar>,
        unifier: &Unifier,
        free_vars: impl Fn(&Type) -> Vec<TypeVar>,
    ) {
        if self.by_binding.is_empty() {
            return;
        }
        let mut position = 0;
        while let Some(var) = vars.get(position).copied() {
            for found in self.reads(unifier.bounds(var)).flat_map(&free_vars) {
                if !vars.contains(&found) {
                    vars.push(found);
                }
            }
            position += 1;
        }
    }

    pub(super) fn add_read_generics<'p>(
        &self,
        mentioned: &mut MentionedGenerics,
        param_of: impl Fn(GenericId) -> Option<&'p GenericParam>,
        unifier: &Unifier,
    ) {
        if self.by_binding.is_empty() {
            return;
        }
        let mut position = 0;
        while let Some(param) = mentioned.get(position).and_then(&param_of) {
            for read in self.reads(&param.bounds) {
                mentioned.collect(&unifier.resolve(read));
            }
            position += 1;
        }
    }

    fn reads<'b>(&'b self, bounds: &'b [Bound]) -> impl Iterator<Item = &'b Type> {
        (bounds.iter())
            .filter(|bound| self.by_binding.contains_key(&bound.binding))
            .filter_map(|bound| bound.argument.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::FieldTraits;
    use crate::core::BindingId;
    use crate::types::mentioned_generics::MentionedGenerics;
    use crate::types::unifier::Unifier;
    use crate::types::{Bound, GenericId, GenericParam, Type, TypeVar};

    const HAS_NAME: BindingId = BindingId::from_index(0);
    const DISPLAY: BindingId = BindingId::from_index(1);

    struct FieldTraitsFixture {
        field_traits: FieldTraits,
        unifier: Unifier,
    }

    impl FieldTraitsFixture {
        fn new() -> Self {
            let mut field_traits = FieldTraits::default();
            field_traits.record("name".into(), HAS_NAME);
            Self {
                field_traits,
                unifier: Unifier::default(),
            }
        }

        fn fresh_var(&mut self) -> TypeVar {
            let Type::Var(var) = self.unifier.fresh() else {
                unreachable!("a fresh type is a variable");
            };
            var
        }

        fn param(index: usize, bounds: Vec<Bound>) -> GenericParam {
            GenericParam {
                id: GenericId::new(index),
                name: "A".into(),
                bounds,
            }
        }
    }

    #[test]
    fn finds_a_field_trait_by_field_and_by_binding() {
        let fixture = FieldTraitsFixture::new();
        assert_eq!(fixture.field_traits.trait_of("name"), Some(HAS_NAME));
        assert_eq!(fixture.field_traits.trait_of("age"), None);
        assert_eq!(
            fixture
                .field_traits
                .field_of(HAS_NAME)
                .map(|field| &**field),
            Some("name")
        );
    }

    #[test]
    fn follows_the_read_of_a_field_bound_to_its_variables() {
        let mut fixture = FieldTraitsFixture::new();
        let owner = fixture.fresh_var();
        let read = fixture.fresh_var();
        let unrelated = fixture.fresh_var();
        (fixture.unifier).add_bound(owner, Bound::with_argument(HAS_NAME, Type::Var(read)));
        (fixture.unifier).add_bound(owner, Bound::with_argument(DISPLAY, Type::Var(unrelated)));
        let mut vars = vec![owner];
        let free_vars = |value: &Type| match value {
            Type::Var(var) => vec![*var],
            _ => Vec::new(),
        };
        (fixture.field_traits).add_read_vars(&mut vars, &fixture.unifier, free_vars);
        assert_eq!(vars, vec![owner, read]);
    }

    #[test]
    fn follows_the_read_of_a_field_bound_to_its_generics() {
        let fixture = FieldTraitsFixture::new();
        let owner = FieldTraitsFixture::param(
            0,
            vec![Bound::with_argument(
                HAS_NAME,
                Type::Generic(GenericId::new(1)),
            )],
        );
        let read = FieldTraitsFixture::param(1, Vec::new());
        let params = [owner, read];
        let mut mentioned = MentionedGenerics::of(&Type::Generic(GenericId::new(0)));
        let param_of = |id: GenericId| params.iter().find(|param| param.id == id);
        (fixture.field_traits).add_read_generics(&mut mentioned, param_of, &fixture.unifier);
        assert_eq!(
            mentioned.into_ids(),
            vec![GenericId::new(0), GenericId::new(1)]
        );
    }
}

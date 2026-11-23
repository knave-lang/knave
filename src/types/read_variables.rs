use super::field_trait_bindings::FieldTraitBindings;
use super::mentioned_generics::MentionedGenerics;
use super::ty::{Bound, GenericId, GenericParam, Type, TypeVar};
use super::unifier::Unifier;

pub(super) struct ReadVariables<'r> {
    bindings: &'r FieldTraitBindings,
    unifier: &'r Unifier,
}

impl<'r> ReadVariables<'r> {
    pub(super) fn new(bindings: &'r FieldTraitBindings, unifier: &'r Unifier) -> Self {
        Self { bindings, unifier }
    }

    pub(super) fn add_vars(
        &self,
        vars: &mut Vec<TypeVar>,
        free_vars: impl Fn(&Type) -> Vec<TypeVar>,
    ) {
        if self.bindings.is_empty() {
            return;
        }
        let mut position = 0;
        while let Some(var) = vars.get(position).copied() {
            for found in self.reads(self.unifier.bounds(var)).flat_map(&free_vars) {
                if !vars.contains(&found) {
                    vars.push(found);
                }
            }
            position += 1;
        }
    }

    pub(super) fn add_generics<'p>(
        &self,
        mentioned: &mut MentionedGenerics,
        param_of: impl Fn(GenericId) -> Option<&'p GenericParam>,
    ) {
        if self.bindings.is_empty() {
            return;
        }
        let mut position = 0;
        while let Some(param) = mentioned.get(position).and_then(&param_of) {
            for read in self.reads(&param.bounds) {
                mentioned.collect(&self.unifier.resolve(read));
            }
            position += 1;
        }
    }

    fn reads<'b>(&'b self, bounds: &'b [Bound]) -> impl Iterator<Item = &'b Type> {
        (bounds.iter())
            .filter(|bound| self.bindings.is_field_bound(bound))
            .filter_map(|bound| bound.argument.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::ReadVariables;
    use crate::core::BindingId;
    use crate::types::field_trait_bindings::FieldTraitBindings;
    use crate::types::mentioned_generics::MentionedGenerics;
    use crate::types::unifier::Unifier;
    use crate::types::{Bound, GenericId, GenericParam, Type, TypeVar};

    const HAS_NAME: BindingId = BindingId::from_index(0);
    const DISPLAY: BindingId = BindingId::from_index(1);

    struct ReadVariablesFixture {
        bindings: FieldTraitBindings,
        unifier: Unifier,
    }

    impl ReadVariablesFixture {
        fn new() -> Self {
            let mut bindings = FieldTraitBindings::default();
            bindings.record("name".into(), HAS_NAME);
            Self {
                bindings,
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

        fn variables(&self) -> ReadVariables<'_> {
            ReadVariables::new(&self.bindings, &self.unifier)
        }
    }

    #[test]
    fn follows_the_read_of_a_field_bound_to_its_variables() {
        let mut fixture = ReadVariablesFixture::new();
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
        fixture.variables().add_vars(&mut vars, free_vars);
        assert_eq!(vars, vec![owner, read]);
    }

    #[test]
    fn follows_the_read_of_a_field_bound_to_its_generics() {
        let fixture = ReadVariablesFixture::new();
        let owner = ReadVariablesFixture::param(
            0,
            vec![Bound::with_argument(
                HAS_NAME,
                Type::Generic(GenericId::new(1)),
            )],
        );
        let read = ReadVariablesFixture::param(1, Vec::new());
        let params = [owner, read];
        let mut mentioned = MentionedGenerics::of(&Type::Generic(GenericId::new(0)));
        let param_of = |id: GenericId| params.iter().find(|param| param.id == id);
        fixture.variables().add_generics(&mut mentioned, param_of);
        assert_eq!(
            mentioned.into_ids(),
            vec![GenericId::new(0), GenericId::new(1)]
        );
    }
}

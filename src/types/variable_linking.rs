use super::TraitTable;
use super::trait_arguments::TraitArguments;
use super::ty::{Type, TypeVar};
use super::type_variables::TypeVariables;
use super::unifier::Unifier;
use crate::source::Span;

pub(super) struct VariableLinking<'a> {
    unifier: &'a mut Unifier,
    traits: &'a TraitTable,
}

impl<'a> VariableLinking<'a> {
    pub(super) fn new(unifier: &'a mut Unifier, traits: &'a TraitTable) -> Self {
        Self { unifier, traits }
    }

    pub(super) fn link(&mut self, vars: &[TypeVar], span: Span) {
        while self.link_round(vars, span) {}
    }

    fn link_round(&mut self, vars: &[TypeVar], span: Span) -> bool {
        let mut linked = false;
        for var in vars {
            if self.variables().is_unbound(*var) {
                linked |= self.link_var(*var, vars, span);
            }
        }
        linked
    }

    fn link_var(&mut self, var: TypeVar, vars: &[TypeVar], span: Span) -> bool {
        let mut linked = false;
        for other in self.linked_vars(var, vars) {
            if self.variables().is_unbound(var)
                && self.variables().is_unbound(other)
                && self
                    .unifier
                    .unify(&Type::Var(var), &Type::Var(other), span)
                    .is_ok()
            {
                linked = true;
            }
        }
        linked
    }

    fn linked_vars(&self, var: TypeVar, vars: &[TypeVar]) -> Vec<TypeVar> {
        let arguments = TraitArguments::of(self.traits);
        self.unifier
            .bounds(var)
            .iter()
            .filter(|bound| arguments.links(bound.binding))
            .filter_map(|bound| {
                let argument = bound
                    .argument
                    .as_ref()
                    .map(|argument| self.unifier.shallow(argument));
                match argument {
                    Some(Type::Var(other)) if other != var && vars.contains(&other) => Some(other),
                    _ => None,
                }
            })
            .collect()
    }

    fn variables(&self) -> TypeVariables<'_> {
        TypeVariables::new(self.unifier)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Type;
    use crate::types::generalization_fixture::GeneralizationFixture;

    #[test]
    fn a_bound_that_links_its_argument_joins_two_variables() {
        let mut fixture = GeneralizationFixture::new();
        let (left, right) = (fixture.var(), fixture.var());
        fixture.bound_by(left, "partial-eq", Some(Type::Var(right)));
        fixture
            .variable_linking()
            .link(&[left, right], GeneralizationFixture::span());
        assert_eq!(fixture.resolve(left), fixture.resolve(right));
    }

    #[test]
    fn a_bound_that_does_not_link_its_argument_leaves_both_apart() {
        let mut fixture = GeneralizationFixture::new();
        let (left, right) = (fixture.var(), fixture.var());
        fixture.bound_by(
            left,
            GeneralizationFixture::UNLINKING_TRAIT,
            Some(Type::Var(right)),
        );
        fixture
            .variable_linking()
            .link(&[left, right], GeneralizationFixture::span());
        assert_ne!(fixture.resolve(left), fixture.resolve(right));
    }

    #[test]
    fn a_variable_outside_the_component_stays_apart() {
        let mut fixture = GeneralizationFixture::new();
        let (inside, outside) = (fixture.var(), fixture.var());
        fixture.bound_by(inside, "partial-eq", Some(Type::Var(outside)));
        fixture
            .variable_linking()
            .link(&[inside], GeneralizationFixture::span());
        assert_ne!(fixture.resolve(inside), fixture.resolve(outside));
    }

    #[test]
    fn a_variable_bound_already_is_not_linked() {
        let mut fixture = GeneralizationFixture::new();
        let (left, right) = (fixture.var(), fixture.var());
        fixture.bound_by(left, "partial-eq", Some(Type::Var(right)));
        fixture.bind(left, GeneralizationFixture::int());
        fixture
            .variable_linking()
            .link(&[left, right], GeneralizationFixture::span());
        assert_eq!(fixture.resolve(right), Type::Var(right));
    }

    #[test]
    fn a_link_made_late_in_a_round_is_followed_in_the_next_round() {
        let mut fixture = GeneralizationFixture::new();
        let (first, second, third) = (fixture.var(), fixture.var(), fixture.var());
        fixture.bound_by(second, "partial-eq", Some(Type::Var(first)));
        fixture.bound_by(second, "partial-eq", Some(Type::Var(third)));
        fixture
            .variable_linking()
            .link(&[first, second, third], GeneralizationFixture::span());
        assert_eq!(fixture.resolve(third), fixture.resolve(second));
    }

    #[test]
    fn a_joined_variable_leaves_its_other_links_to_the_variable_it_joined() {
        let mut fixture = GeneralizationFixture::new();
        let (first, second, third, fourth) =
            (fixture.var(), fixture.var(), fixture.var(), fixture.var());
        fixture.bound_by(first, "partial-eq", Some(Type::Var(second)));
        fixture.bound_by(first, "partial-eq", Some(Type::Var(third)));
        fixture.bound_by(second, "partial-eq", Some(Type::Var(fourth)));
        fixture.variable_linking().link(
            &[first, second, third, fourth],
            GeneralizationFixture::span(),
        );
        assert_eq!(fixture.resolve(first), Type::Var(third));
    }
}

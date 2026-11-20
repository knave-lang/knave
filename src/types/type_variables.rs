use super::ty::{Bound, Type, TypeVar};
use super::unifier::Unifier;

#[derive(Clone, Copy)]
pub(super) struct TypeVariables<'a> {
    unifier: &'a Unifier,
}

impl<'a> TypeVariables<'a> {
    pub(super) fn new(unifier: &'a Unifier) -> Self {
        Self { unifier }
    }

    pub(super) fn free_in(&self, value: &Type) -> Vec<TypeVar> {
        let mut vars = Vec::new();
        Self::collect(&self.unifier.resolve(value), &mut vars);
        vars
    }

    pub(super) fn is_unbound(&self, var: TypeVar) -> bool {
        matches!(self.unifier.shallow(&Type::Var(var)), Type::Var(current) if current == var)
    }

    pub(super) fn self_plain_bounds(&self, var: TypeVar) -> Vec<Bound> {
        let mut bounds = Vec::new();
        for bound in self.unifier.bounds(var) {
            let bound = match &bound.argument {
                Some(argument) => {
                    let argument = self.unifier.resolve(argument);
                    if argument == Type::Var(var) {
                        Bound::plain(bound.binding)
                    } else {
                        Bound::with_argument(bound.binding, argument)
                    }
                }
                None => bound.clone(),
            };
            if !bounds.contains(&bound) {
                bounds.push(bound);
            }
        }
        bounds
    }

    fn collect(value: &Type, vars: &mut Vec<TypeVar>) {
        match value {
            Type::Var(var) if !vars.contains(var) => vars.push(*var),
            Type::Function(_, parameters, result) => {
                parameters
                    .iter()
                    .for_each(|parameter| Self::collect(parameter, vars));
                Self::collect(result, vars);
            }
            Type::Named(_, arguments) => arguments
                .iter()
                .for_each(|argument| Self::collect(argument, vars)),
            Type::Ref(_, inner) | Type::Mut(_, inner) => Self::collect(inner, vars),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::generalization_fixture::GeneralizationFixture;
    use crate::types::{ClosureKind, Type};

    #[test]
    fn a_variable_written_twice_is_free_once() {
        let mut fixture = GeneralizationFixture::new();
        let var = fixture.var();
        let function = Type::Function(
            ClosureKind::Fn,
            vec![Type::Var(var), Type::Var(var)],
            Box::new(Type::Var(var)),
        );
        assert_eq!(fixture.variables().free_in(&function), vec![var]);
    }

    #[test]
    fn a_bound_variable_is_free_through_its_binding() {
        let mut fixture = GeneralizationFixture::new();
        let (bound, target) = (fixture.var(), fixture.var());
        fixture.bind(bound, Type::Var(target));
        assert_eq!(fixture.variables().free_in(&Type::Var(bound)), vec![target]);
    }

    #[test]
    fn every_part_of_a_type_holds_free_variables() {
        let mut fixture = GeneralizationFixture::new();
        let (parameter, result, argument) = (fixture.var(), fixture.var(), fixture.var());
        let named = Type::Named(
            GeneralizationFixture::named_binding(),
            vec![Type::Var(argument)],
        );
        let function = Type::Function(
            ClosureKind::Fn,
            vec![Type::shared(Type::Var(parameter)), named],
            Box::new(Type::Var(result)),
        );
        assert_eq!(
            fixture.variables().free_in(&function),
            vec![parameter, argument, result]
        );
    }

    #[test]
    fn a_fresh_variable_is_unbound() {
        let mut fixture = GeneralizationFixture::new();
        let var = fixture.var();
        assert!(fixture.variables().is_unbound(var));
    }

    #[test]
    fn a_variable_bound_to_another_is_not_unbound() {
        let mut fixture = GeneralizationFixture::new();
        let (bound, target) = (fixture.var(), fixture.var());
        fixture.bind(bound, Type::Var(target));
        assert!(!fixture.variables().is_unbound(bound));
    }

    #[test]
    fn a_bound_on_the_variable_itself_becomes_plain() {
        let mut fixture = GeneralizationFixture::new();
        let var = fixture.var();
        fixture.bound_by(var, "partial-eq", Some(Type::Var(var)));
        let plain = fixture.bound_of("partial-eq", None);
        assert_eq!(fixture.variables().self_plain_bounds(var), vec![plain]);
    }

    #[test]
    fn a_bound_on_another_type_keeps_its_argument() {
        let mut fixture = GeneralizationFixture::new();
        let var = fixture.var();
        fixture.bound_by(var, "partial-eq", Some(GeneralizationFixture::int()));
        let kept = fixture.bound_of("partial-eq", Some(GeneralizationFixture::int()));
        assert_eq!(fixture.variables().self_plain_bounds(var), vec![kept]);
    }

    #[test]
    fn a_repeated_bound_is_listed_once() {
        let mut fixture = GeneralizationFixture::new();
        let var = fixture.var();
        fixture.bound_by(var, "partial-eq", Some(Type::Var(var)));
        fixture.bound_by(var, "partial-eq", None);
        assert_eq!(fixture.variables().self_plain_bounds(var).len(), 1);
    }
}

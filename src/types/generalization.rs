use super::generalization_rules::GeneralizationRules;
use super::generic_declarations::GenericDeclarations;
use super::generic_registry::GenericRegistry;
use super::parameter_names::ParameterNames;
use super::ty::{GenericParam, Type, TypeVar};
use super::type_variables::TypeVariables;
use super::unifier::{LiteralKind, Unifier};
use crate::source::Span;

pub(super) struct Generalization<'a> {
    unifier: &'a mut Unifier,
    generics: &'a mut GenericRegistry,
    span: Span,
}

impl<'a> Generalization<'a> {
    pub(super) fn new(
        unifier: &'a mut Unifier,
        generics: &'a mut GenericRegistry,
        span: Span,
    ) -> Self {
        Self {
            unifier,
            generics,
            span,
        }
    }

    pub(super) fn generalize(
        &mut self,
        vars: Vec<TypeVar>,
        start: usize,
        rules: &GeneralizationRules,
    ) {
        let mut position = start;
        for var in vars {
            if self.generalize_var(var, position, rules).is_some() {
                position += 1;
            }
        }
    }

    pub(super) fn default_literal(&mut self, var: TypeVar, literal: LiteralKind) {
        let defaulted = self.unifier.unify(
            &Type::Var(var),
            &Type::Primitive(literal.default_type()),
            self.span,
        );
        debug_assert!(
            defaulted.is_ok(),
            "a literal variable accepts its default type"
        );
    }

    fn generalize_var(
        &mut self,
        var: TypeVar,
        position: usize,
        rules: &GeneralizationRules,
    ) -> Option<GenericParam> {
        if !self.variables().is_unbound(var) {
            return None;
        }
        let literal = self.unifier.literal_kind(var);
        if let Some(literal) = literal
            && rules.defaults_literal(var)
        {
            self.default_literal(var, literal);
            return None;
        }
        let name = ParameterNames::at(position);
        let mut bounds = rules.generic_bounds(self.variables().self_plain_bounds(var));
        bounds
            .iter_mut()
            .for_each(|bound| bound.origins.generalize());
        let param =
            GenericDeclarations::new(self.generics, self.unifier).generalize(var, &name, bounds);
        if let Some(literal) = literal {
            self.generics.mark_literal(param.id, literal);
        }
        Some(param)
    }

    fn variables(&self) -> TypeVariables<'_> {
        TypeVariables::new(self.unifier)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Type;
    use crate::types::generalization_fixture::GeneralizationFixture;
    use crate::types::literal_parameters::LiteralParameters;
    use crate::types::unifier::LiteralKind;

    #[test]
    fn each_open_variable_becomes_the_next_named_generic() {
        let mut fixture = GeneralizationFixture::new();
        let (first, second) = (fixture.var(), fixture.var());
        let rules = GeneralizationFixture::rules(&[], LiteralParameters::Default);
        fixture
            .generalization()
            .generalize(vec![first, second], 0, &rules);
        let names =
            [first, second].map(|var| fixture.generic_of(var).map(|param| param.name.to_string()));
        assert_eq!(names, [Some("a".to_string()), Some("b".to_string())]);
    }

    #[test]
    fn a_bound_variable_takes_no_name() {
        let mut fixture = GeneralizationFixture::new();
        let (bound, open) = (fixture.var(), fixture.var());
        fixture.bind(bound, GeneralizationFixture::int());
        let rules = GeneralizationFixture::rules(&[], LiteralParameters::Default);
        fixture
            .generalization()
            .generalize(vec![bound, open], 0, &rules);
        let name = fixture.generic_of(open).map(|param| param.name.to_string());
        assert_eq!(name, Some("a".to_string()));
    }

    #[test]
    fn a_defaulted_literal_takes_its_default_type_and_no_name() {
        let mut fixture = GeneralizationFixture::new();
        let (literal, open) = (fixture.literal_var(), fixture.var());
        let rules = GeneralizationFixture::rules(&[], LiteralParameters::Default);
        fixture
            .generalization()
            .generalize(vec![literal, open], 0, &rules);
        let name = fixture.generic_of(open).map(|param| param.name.to_string());
        assert_eq!(
            (fixture.resolve(literal), name),
            (GeneralizationFixture::int(), Some("a".to_string()))
        );
    }

    #[test]
    fn a_literal_kept_open_becomes_a_literal_generic() {
        let mut fixture = GeneralizationFixture::new();
        let literal = fixture.literal_var();
        let rules = GeneralizationFixture::rules(&[literal], LiteralParameters::KeepOpen);
        fixture
            .generalization()
            .generalize(vec![literal], 0, &rules);
        assert_eq!(fixture.literal_of(literal), Some(LiteralKind::Integer));
    }

    #[test]
    fn a_generic_takes_the_plain_bounds_of_its_variable() {
        let mut fixture = GeneralizationFixture::new();
        let var = fixture.var();
        fixture.bound_by(var, "partial-eq", Some(Type::Var(var)));
        let rules = GeneralizationFixture::rules(&[], LiteralParameters::Default);
        fixture.generalization().generalize(vec![var], 0, &rules);
        let bounds = fixture.generic_of(var).map(|param| param.bounds.clone());
        assert_eq!(bounds, Some(vec![fixture.bound_of("partial-eq", None)]));
    }

    #[test]
    fn a_defaulted_literal_takes_its_default_type() {
        let mut fixture = GeneralizationFixture::new();
        let literal = fixture.literal_var();
        fixture
            .generalization()
            .default_literal(literal, LiteralKind::Integer);
        assert_eq!(fixture.resolve(literal), GeneralizationFixture::int());
    }
}

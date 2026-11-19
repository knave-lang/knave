use super::expectation::Expectation;
use super::passing_facts::PassingFacts;
use super::passing_tables::PassingTables;
use super::ty::{Type, TypeVar};
use super::unification::Unification;
use super::unifier::Unifier;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::typed::Borrow;

pub(super) struct BorrowRules<'s, 'a> {
    tables: PassingTables<'a>,
    unifier: &'s mut Unifier,
}

impl<'s, 'a> BorrowRules<'s, 'a> {
    pub(super) fn new(tables: PassingTables<'a>, unifier: &'s mut Unifier) -> Self {
        Self { tables, unifier }
    }

    pub(super) fn describe(&mut self, described: &Type) -> String {
        self.unification().describe(described)
    }

    pub(super) fn pass_types(
        &mut self,
        parameter: &Type,
        argument: &Type,
        span: Span,
    ) -> Result<Borrow, Diagnostic> {
        match (parameter, argument) {
            (expected, Type::Ref(_, _)) if Self::is_known_value(expected) => {
                let levels = argument.reference_depth();
                let referent = argument.innermost_referent().clone();
                self.unification()
                    .expect(Expectation::plain(expected), &referent, span)?;
                Ok(self.borrow_of_referent(&referent, levels))
            }
            (Type::Ref(_, _), Type::Ref(_, _)) if self.collapses(parameter, argument) => {
                let parameter = self.unifier.resolve(parameter);
                let extra = argument.reference_depth() - parameter.reference_depth();
                let stripped = Self::without_references(argument, extra);
                self.fit(&parameter, &stripped, span)
                    .map(|()| Borrow::Collapse(extra))
            }
            (Type::Var(var), Type::Ref(_, _)) => self.pass_to_variable(*var, argument, span),
            (Type::Mut(_, _), Type::Mut(_, _)) => self
                .fit(parameter, argument, span)
                .map(|()| Borrow::Reborrow),
            (Type::Ref(_, _), Type::Mut(_, _))
                if self.unification().coerces(parameter, argument, span) =>
            {
                self.fit(parameter, argument, span)
                    .map(|()| Borrow::Reborrow)
            }
            (expected, found) => self.pass_value(expected, found, span),
        }
    }

    fn pass_value(
        &mut self,
        expected: &Type,
        found: &Type,
        span: Span,
    ) -> Result<Borrow, Diagnostic> {
        if expected.is_unsized() && matches!(found, Type::Var(_)) {
            let borrowed = Type::shared(expected.clone());
            return self.fit(&borrowed, found, span).map(|()| Borrow::None);
        }
        self.fit(expected, found, span).map(|()| Borrow::None)
    }

    fn collapses(&self, parameter: &Type, argument: &Type) -> bool {
        let parameter = self.unifier.resolve(parameter);
        argument.reference_depth() > parameter.reference_depth()
    }

    pub(super) fn without_references(value: &Type, levels: usize) -> Type {
        match value {
            Type::Ref(_, inner) if levels > 0 => Self::without_references(inner, levels - 1),
            _ => value.clone(),
        }
    }

    fn borrow_of_referent(&self, referent: &Type, levels: usize) -> Borrow {
        if self
            .tables
            .copy_rules()
            .is_small_copy(&self.unifier.resolve(referent))
        {
            Borrow::Deref(levels)
        } else {
            Borrow::Passthrough
        }
    }

    fn pass_to_variable(
        &mut self,
        var: TypeVar,
        argument: &Type,
        span: Span,
    ) -> Result<Borrow, Diagnostic> {
        let mut levels = 0;
        let mut passed = argument;
        while let Type::Ref(_, referent) = passed
            && self.dereferences(var, passed, referent)
        {
            levels += 1;
            passed = referent;
        }
        if levels == 0
            && let Type::Ref(_, referent) = argument
            && self.passes_through(var, argument, referent)
        {
            return self
                .fit(&Type::Var(var), referent, span)
                .map(|()| Borrow::Passthrough);
        }
        let borrow = if levels == 0 {
            Borrow::None
        } else {
            Borrow::Deref(levels)
        };
        self.fit(&Type::Var(var), &passed.clone(), span)
            .map(|()| borrow)
    }

    fn dereferences(&self, var: TypeVar, reference: &Type, referent: &Type) -> bool {
        let fails_traits =
            self.unifier.literal_kind(var).is_some() || self.fails_bounds(var, reference);
        fails_traits && self.facts().holds_copy_value(referent)
    }

    fn passes_through(&self, var: TypeVar, reference: &Type, referent: &Type) -> bool {
        self.fails_bounds(var, reference) && !self.fails_bounds(var, referent)
    }

    fn fails_bounds(&self, var: TypeVar, value: &Type) -> bool {
        self.unifier.bounds(var).iter().any(|bound| {
            !self.tables.declarations.traits().table.satisfies(
                bound,
                value,
                &|generic, required| self.unifier.generic_bounds(generic).contains(required),
            )
        })
    }

    pub(super) fn fit(
        &mut self,
        expected: &Type,
        found: &Type,
        span: Span,
    ) -> Result<(), Diagnostic> {
        self.unification()
            .expect(Expectation::plain(expected), found, span)
    }

    fn is_known_value(parameter: &Type) -> bool {
        matches!(parameter, Type::Named(..) | Type::Primitive(_))
    }

    fn unification(&mut self) -> Unification<'_> {
        self.tables.unification(self.unifier)
    }

    fn facts(&self) -> PassingFacts<'_> {
        PassingFacts::new(self.tables, self.unifier)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Type;
    use crate::types::passing_fixture::PassingFixture;
    use crate::types::ty::{Bound, GenericId};
    use crate::types::typed::Borrow;

    #[test]
    fn a_borrowed_small_copy_passes_dereferenced_to_a_value() {
        let borrowed = Type::shared(PassingFixture::int());
        assert_eq!(
            PassingFixture::new().passed(&PassingFixture::int(), &borrowed),
            Borrow::Deref(1)
        );
    }

    #[test]
    fn a_borrowed_string_passes_through_to_a_value() {
        let borrowed = Type::shared(PassingFixture::string());
        assert_eq!(
            PassingFixture::new().passed(&PassingFixture::string(), &borrowed),
            Borrow::Passthrough
        );
    }

    #[test]
    fn a_deeper_reference_collapses_to_the_parameter() {
        let parameter = Type::shared(PassingFixture::string());
        let argument = Type::shared(parameter.clone());
        assert_eq!(
            PassingFixture::new().passed(&parameter, &argument),
            Borrow::Collapse(1)
        );
    }

    #[test]
    fn a_mut_reborrows_into_a_mut() {
        let changed = Type::mutable(PassingFixture::int());
        assert_eq!(
            PassingFixture::new().passed(&changed, &changed),
            Borrow::Reborrow
        );
    }

    #[test]
    fn a_mut_reborrows_into_a_ref() {
        let parameter = Type::shared(PassingFixture::int());
        let argument = Type::mutable(PassingFixture::int());
        assert_eq!(
            PassingFixture::new().passed(&parameter, &argument),
            Borrow::Reborrow
        );
    }

    #[test]
    fn a_literal_variable_takes_a_borrowed_value_dereferenced() {
        let mut fixture = PassingFixture::new();
        let literal = fixture.literal();
        let borrowed = Type::shared(PassingFixture::int());
        let borrow = (fixture.rules()).pass_types(&literal, &borrowed, PassingFixture::span());
        assert_eq!(borrow.expect("the types fit"), Borrow::Deref(1));
    }

    #[test]
    fn a_bounded_variable_takes_a_borrow_of_a_generic_with_the_bound_through_it() {
        let mut fixture = PassingFixture::new();
        let number = Bound::plain(fixture.trait_named("number"));
        let generic = GenericId::new(0);
        fixture
            .unifier()
            .register_generic(generic, vec![number.clone()]);
        let borrowed = Type::shared(Type::Generic(generic));
        assert_eq!(
            fixture.passed_to_variable_bounded_by(number, &borrowed),
            Borrow::Passthrough
        );
    }

    #[test]
    fn a_bounded_variable_takes_a_borrow_of_a_generic_without_the_bound_as_is() {
        let mut fixture = PassingFixture::new();
        let number = Bound::plain(fixture.trait_named("number"));
        let generic = GenericId::new(0);
        fixture.unifier().register_generic(generic, Vec::new());
        let borrowed = Type::shared(Type::Generic(generic));
        assert_eq!(
            fixture.passed_to_variable_bounded_by(number, &borrowed),
            Borrow::None
        );
    }

    #[test]
    fn an_unsized_parameter_borrows_an_open_value() {
        let mut fixture = PassingFixture::new();
        let open = fixture.unifier().fresh();
        let passed =
            (fixture.rules()).pass_types(&PassingFixture::str(), &open, PassingFixture::span());
        assert_eq!(passed.expect("the types fit"), Borrow::None);
        let resolved = fixture.unifier().resolve(&open);
        assert_eq!(resolved, Type::shared(PassingFixture::str()));
    }

    #[test]
    fn a_value_of_another_type_does_not_fit() {
        let mut fixture = PassingFixture::new();
        let fitted = (fixture.rules()).fit(
            &PassingFixture::int(),
            &PassingFixture::string(),
            PassingFixture::span(),
        );
        assert!(fitted.is_err());
    }
}

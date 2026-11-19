use super::borrow_rules::BorrowRules;
use super::copy_requirement::CopyRequirement;
use super::declared_types::DeclaredTypes;
use super::passed_types::PassedTypes;
use super::passing_facts::PassingFacts;
use super::passing_sources::PassingSources;
use super::pending::PendingBorrow;
use super::ty::{BoundOrigin, Type, TypeVar};
use super::unifier::Unifier;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::typed::Borrow;

pub(super) struct PendingBorrowDecision<'s, 'a> {
    sources: PassingSources<'a>,
    unifier: &'s mut Unifier,
}

impl<'s, 'a> PendingBorrowDecision<'s, 'a> {
    pub(super) fn new(sources: PassingSources<'a>, unifier: &'s mut Unifier) -> Self {
        Self { sources, unifier }
    }

    pub(super) fn decide(&mut self, pending: &PendingBorrow) -> Result<Borrow, Diagnostic> {
        let argument = self.unifier.resolve(&pending.argument);
        let parameter = self.unifier.shallow(&Type::Var(pending.var));
        if let Type::Var(var) = parameter
            && self.facts().referent_unknown(&argument)
            && self.needs_value(var)
        {
            let levels = argument.reference_depth();
            let referent = argument.innermost_referent().clone();
            return self
                .rules()
                .fit(&parameter, &referent, pending.span)
                .map(|()| Borrow::Deref(levels));
        }
        if Self::borrows_open_parameter_type(&parameter, &argument) {
            return Ok(self.copy_through_borrow(&parameter, &argument, pending.span));
        }
        match self.view_for_unknown_referent(&parameter, &argument) {
            Some(view) => {
                let passed = PassedTypes {
                    parameter: &parameter,
                    argument: &argument,
                };
                self.pass_as_view(&view, &passed, pending.span)
            }
            None => self.rules().pass_types(&parameter, &argument, pending.span),
        }
    }

    fn view_for_unknown_referent(&self, parameter: &Type, argument: &Type) -> Option<Type> {
        if !self.facts().referent_unknown(argument) {
            return None;
        }
        self.borrowed_view(parameter)
    }

    fn pass_as_view(
        &mut self,
        view: &Type,
        passed: &PassedTypes<'_>,
        span: Span,
    ) -> Result<Borrow, Diagnostic> {
        let referent = passed.argument.innermost_referent();
        self.rules().fit(view, referent, span)?;
        let argument = self.unifier.resolve(passed.argument);
        let parameter = self.unifier.resolve(passed.parameter);
        let extra = argument
            .reference_depth()
            .saturating_sub(parameter.reference_depth());
        let stripped = BorrowRules::without_references(&argument, extra);
        self.rules()
            .fit(&parameter, &stripped, span)
            .map(|()| Borrow::Deref(extra))
    }

    fn borrowed_view(&self, borrowed: &Type) -> Option<Type> {
        let view = self.unifier.resolve(borrowed);
        match &view {
            Type::Ref(_, referent)
                if DeclaredTypes::new(self.sources.declarations.types()).is_unsized(referent) =>
            {
                Some(view)
            }
            _ => None,
        }
    }

    fn borrows_open_parameter_type(parameter: &Type, argument: &Type) -> bool {
        matches!(parameter, Type::Var(_))
            && argument.is_reference()
            && argument.innermost_referent() == parameter
    }

    fn copy_through_borrow(&mut self, parameter: &Type, argument: &Type, copied: Span) -> Borrow {
        CopyRequirement::new(self.sources.copy_trait, self.unifier)
            .require(parameter, vec![BoundOrigin::BorrowCopied(copied)]);
        Borrow::Deref(argument.reference_depth())
    }

    fn needs_value(&self, var: TypeVar) -> bool {
        self.unifier.literal_kind(var).is_some() || self.facts().allows_only_copy_values(var)
    }

    fn rules(&mut self) -> BorrowRules<'_, 'a> {
        BorrowRules::new(self.sources, self.unifier)
    }

    fn facts(&self) -> PassingFacts<'_> {
        PassingFacts::new(self.sources, self.unifier)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Type;
    use crate::types::passing_fixture::PassingFixture;
    use crate::types::pending::PendingBorrow;
    use crate::types::typed::Borrow;

    fn pending(var: &Type, argument: Type) -> PendingBorrow {
        let Type::Var(var) = var else {
            unreachable!("a pending borrow waits on a variable")
        };
        PendingBorrow {
            var: *var,
            argument,
            span: PassingFixture::span(),
        }
    }

    #[test]
    fn a_literal_parameter_takes_an_unknown_referent_by_value() {
        let mut fixture = PassingFixture::new();
        let parameter = fixture.literal();
        let argument = Type::shared(fixture.unifier().fresh());
        let decided = fixture
            .argument_passing()
            .decide(&pending(&parameter, argument));
        assert_eq!(decided.expect("the borrow is decided"), Borrow::Deref(1));
    }

    #[test]
    fn a_borrow_of_the_parameter_type_copies_it() {
        let mut fixture = PassingFixture::new();
        let parameter = fixture.unifier().fresh();
        let argument = Type::shared(parameter.clone());
        let decided = fixture
            .argument_passing()
            .decide(&pending(&parameter, argument));
        assert_eq!(decided.expect("the borrow is decided"), Borrow::Deref(1));
        let Type::Var(var) = parameter else {
            unreachable!("fresh makes a variable")
        };
        let copy = fixture.copy_trait();
        assert!(
            fixture
                .unifier()
                .bounds(var)
                .iter()
                .any(|bound| bound.binding == copy)
        );
    }

    #[test]
    fn a_str_view_parameter_takes_an_unknown_referent_as_a_view() {
        let mut fixture = PassingFixture::new();
        let parameter = fixture.unifier().fresh();
        let view = Type::shared(PassingFixture::str());
        let Type::Var(var) = parameter else {
            unreachable!("fresh makes a variable")
        };
        fixture.unifier().bind(var, view);
        let referent = fixture.unifier().fresh();
        let argument = Type::shared(Type::shared(referent.clone()));
        let decided = fixture
            .argument_passing()
            .decide(&pending(&Type::Var(var), argument));
        assert_eq!(decided.expect("the borrow is decided"), Borrow::Deref(2));
        let view = Type::shared(PassingFixture::str());
        assert_eq!(fixture.unifier().resolve(&referent), view);
    }
}

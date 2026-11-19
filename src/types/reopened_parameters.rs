use super::deferred_decisions::DeferredDecisions;
use super::passing_tables::PassingTables;
use super::reopened_parameter::ReopenedParameter;
use super::ty::Type;
use super::unifier::Unifier;
use crate::source::Span;

pub(super) struct ReopenedParameters<'a> {
    tables: PassingTables<'a>,
    unifier: &'a mut Unifier,
    deferred: &'a mut DeferredDecisions,
}

impl<'a> ReopenedParameters<'a> {
    pub(super) fn new(
        tables: PassingTables<'a>,
        unifier: &'a mut Unifier,
        deferred: &'a mut DeferredDecisions,
    ) -> Self {
        Self {
            tables,
            unifier,
            deferred,
        }
    }

    pub(super) fn reopen_borrowed_copy_parameters(&mut self, pinned: Type, span: Span) -> Type {
        let Type::Function(kind, parameters, result) = pinned else {
            return pinned;
        };
        let parameters = parameters
            .into_iter()
            .map(|parameter| self.reopen_parameter(parameter, span))
            .collect();
        Type::Function(kind, parameters, result)
    }

    pub(super) fn close(&mut self) {
        for reopened in std::mem::take(&mut self.deferred.reopened_parameters) {
            if let Some(pinned) = self.pinned_part_left_open(&reopened) {
                let closed = self
                    .unifier
                    .unify(&reopened.variable, &pinned, reopened.span);
                debug_assert!(
                    closed.is_ok(),
                    "a reopened parameter accepts its pinned type"
                );
            }
        }
    }

    fn reopen_parameter(&mut self, pinned: Type, span: Span) -> Type {
        if !self.borrows_copy_value(&pinned) {
            return pinned;
        }
        let variable = self.unifier.fresh();
        self.deferred.reopened_parameters.push(ReopenedParameter {
            variable: variable.clone(),
            pinned,
            span,
        });
        variable
    }

    fn pinned_part_left_open(&self, reopened: &ReopenedParameter) -> Option<Type> {
        match self.unifier.resolve(&reopened.variable) {
            Type::Var(_) => Some(reopened.pinned.referent().clone()),
            Type::Ref(..) => Some(reopened.pinned.clone()),
            _ => None,
        }
    }

    fn borrows_copy_value(&self, parameter: &Type) -> bool {
        let copy_rules = || self.tables.copy_rules();
        matches!(parameter, Type::Ref(_, referent) if copy_rules().is_small_copy(referent))
    }
}

#[cfg(test)]
mod tests {
    use crate::types::ClosureKind;
    use crate::types::Type;
    use crate::types::passing_fixture::PassingFixture;

    #[test]
    fn a_borrowed_copy_parameter_reopens_and_closes_to_its_pinned_type() {
        let mut fixture = PassingFixture::new();
        let borrowed = Type::shared(PassingFixture::int());
        let pinned = Type::Function(
            ClosureKind::Fn,
            vec![borrowed.clone()],
            Box::new(Type::unit()),
        );
        let reopened = (fixture.reopened_parameters())
            .reopen_borrowed_copy_parameters(pinned, PassingFixture::span());
        let Type::Function(_, parameters, _) = reopened else {
            unreachable!("a function stays a function")
        };
        assert!(matches!(parameters[0], Type::Var(_)));
        assert_eq!(fixture.deferred().reopened_parameters.len(), 1);
        fixture.reopened_parameters().close();
        assert_eq!(
            fixture.unifier().resolve(&parameters[0]),
            PassingFixture::int()
        );
    }

    #[test]
    fn an_owned_parameter_stays_pinned() {
        let mut fixture = PassingFixture::new();
        let pinned = Type::Function(
            ClosureKind::Fn,
            vec![PassingFixture::string()],
            Box::new(Type::unit()),
        );
        let reopened = (fixture.reopened_parameters())
            .reopen_borrowed_copy_parameters(pinned.clone(), PassingFixture::span());
        assert_eq!(reopened, pinned);
    }
}

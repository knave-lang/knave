use super::borrow_rules::BorrowRules;
use super::declared_types::DeclaredTypes;
use super::deferred_decisions::DeferredDecisions;
use super::passed_types::PassedTypes;
use super::passing_facts::PassingFacts;
use super::passing_sources::PassingSources;
use super::pending::PendingBorrow;
use super::pending_borrow_decision::PendingBorrowDecision;
use super::place_types::PlaceTypes;
use super::problems::{MismatchContext, TypeProblems};
use super::ty::{Type, TypeVar};
use super::typed::TypedExpr;
use super::unifier::Unifier;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::typed::Borrow;

pub(super) struct ArgumentPassing<'a> {
    sources: PassingSources<'a>,
    unifier: &'a mut Unifier,
    deferred: &'a mut DeferredDecisions,
}

impl<'a> ArgumentPassing<'a> {
    pub(super) fn new(
        sources: PassingSources<'a>,
        unifier: &'a mut Unifier,
        deferred: &'a mut DeferredDecisions,
    ) -> Self {
        Self {
            sources,
            unifier,
            deferred,
        }
    }

    pub(super) fn fit(
        &mut self,
        expected: &Type,
        found: &Type,
        span: Span,
    ) -> Result<(), Diagnostic> {
        self.rules().fit(expected, found, span)
    }

    pub(super) fn pass_argument(
        &mut self,
        parameter: &Type,
        value: &TypedExpr,
    ) -> Result<Borrow, Diagnostic> {
        let parameter = self.unifier.shallow(parameter);
        let argument = self.slice_argument(&parameter, &value.ty, value.span);
        if self.facts().passes_as_written(value) {
            return self
                .rules()
                .fit(&parameter, &argument, value.span)
                .map(|()| Borrow::None);
        }
        match (&parameter, &argument) {
            (Type::Ref(_, inner), found) if !found.is_reference() => {
                if let Some(help) = self.facts().string_as_str_help(inner, found, value) {
                    return Err(TypeProblems::mismatch(
                        &self.rules().describe(inner),
                        &self.rules().describe(found),
                        value.span,
                        MismatchContext::Plain,
                    )
                    .with_help(help));
                }
                self.rules()
                    .fit(inner, found, value.span)
                    .map(|()| Borrow::Shared)
            }
            (Type::Mut(_, inner), _) => {
                let passed = PassedTypes {
                    parameter: &parameter,
                    argument: &argument,
                };
                self.pass_mut_argument(inner, &passed, value)
            }
            (Type::Var(var), Type::Ref(_, _)) if self.facts().referent_unknown(&argument) => {
                Ok(self.defer_borrow(*var, &argument, value.span))
            }
            (Type::Var(var), Type::Var(_)) if self.dereference_result_unknown(value) => {
                Ok(self.defer_borrow(*var, &argument, value.span))
            }
            _ => self.rules().pass_types(&parameter, &argument, value.span),
        }
    }

    pub(super) fn layers_above_held_view(
        &self,
        callee: Option<&TypedExpr>,
        parameter: &Type,
        value: &TypedExpr,
    ) -> Option<usize> {
        let callee = callee?;
        if !self.may_collapse(parameter) || self.facts().passes_as_written(value) {
            return None;
        }
        let parameter = self.unifier.resolve(parameter);
        let argument = self.unifier.resolve(&value.ty);
        let Type::Var(variable) = parameter.innermost_referent() else {
            return None;
        };
        let layers = argument
            .reference_depth()
            .checked_sub(parameter.reference_depth() + 1)?;
        let holds_view = DeclaredTypes::new(self.sources.declarations.types())
            .is_unsized(argument.innermost_referent())
            && self
                .unifier
                .resolve(&callee.ty)
                .holds_outside_references(&Type::Var(*variable));
        holds_view.then_some(layers)
    }

    pub(super) fn pass_held_view(
        &mut self,
        parameter: &Type,
        value: &TypedExpr,
        layers: usize,
    ) -> Result<Borrow, Diagnostic> {
        let argument = self.unifier.resolve(&value.ty);
        let view = BorrowRules::without_references(&argument, layers);
        let borrow = match layers {
            0 => Borrow::None,
            _ => Borrow::Collapse(layers),
        };
        self.rules()
            .fit(parameter, &view, value.span)
            .map(|()| borrow)
    }

    fn pass_mut_argument(
        &mut self,
        inner: &Type,
        passed: &PassedTypes<'_>,
        value: &TypedExpr,
    ) -> Result<Borrow, Diagnostic> {
        let place = PlaceTypes::new(
            self.unifier,
            DeclaredTypes::new(self.sources.declarations.types()),
        )
        .changed(value);
        if value.is_place() && !place.is_reference() {
            return self
                .rules()
                .fit(inner, &place, value.span)
                .map(|()| Borrow::Mutable);
        }
        if !value.is_place() && !passed.argument.is_reference() {
            return Err(TypeProblems::mut_borrow_of_copy(
                value.span,
                self.facts().field_place(value),
            ));
        }
        let found = if value.is_place() {
            &place
        } else {
            passed.argument
        };
        self.rules().pass_types(passed.parameter, found, value.span)
    }

    fn slice_argument(&mut self, parameter: &Type, argument: &Type, span: Span) -> Type {
        let found = self.unifier.resolve(argument);
        match (parameter, &found) {
            (Type::Ref(_, expected), Type::Ref(_, referent)) => {
                Type::shared(self.vector_before_slice(expected, referent, span))
            }
            (Type::Ref(_, expected), referent) if !referent.is_reference() => {
                self.vector_before_slice(expected, referent, span)
            }
            _ => found,
        }
    }

    fn vector_before_slice(&mut self, expected: &Type, found: &Type, span: Span) -> Type {
        let Type::Named(slice, _) = self.unifier.shallow(expected) else {
            return found.clone();
        };
        let Type::Var(_) = found else {
            return found.clone();
        };
        if !self.sources.declarations.types().rust_types.is_slice(slice) {
            return found.clone();
        }
        let Some(vector) = self
            .sources
            .declarations
            .types()
            .rust_types
            .binding_with_template(crate::types::RustType::VECTOR_TEMPLATE)
        else {
            return found.clone();
        };
        let element = self.unifier.fresh();
        match self
            .unifier
            .unify(found, &Type::Named(vector, vec![element]), span)
        {
            Ok(()) => self.unifier.resolve(found),
            Err(_) => found.clone(),
        }
    }

    fn defer_borrow(&mut self, var: TypeVar, argument: &Type, span: Span) -> Borrow {
        self.deferred.borrows.push(PendingBorrow {
            var,
            argument: argument.clone(),
            span,
        });
        Borrow::Pending(self.deferred.borrows.len() - 1)
    }

    fn may_collapse(&self, parameter: &Type) -> bool {
        matches!(self.unifier.shallow(parameter), Type::Ref(..))
    }

    pub(super) fn decide(&mut self, pending: &PendingBorrow) -> Result<Borrow, Diagnostic> {
        PendingBorrowDecision::new(self.sources, self.unifier).decide(pending)
    }

    fn dereference_result_unknown(&self, value: &TypedExpr) -> bool {
        value.kind.is_dereference()
            && self
                .facts()
                .referent_unknown(&self.unifier.resolve(&value.ty))
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
    use crate::types::typed::Borrow;

    #[test]
    fn an_unknown_referent_defers_its_borrow() {
        let mut fixture = PassingFixture::new();
        let parameter = fixture.unifier().fresh();
        let open = fixture.unifier().fresh();
        let value = fixture.call_result(Type::shared(open));
        let passed = fixture.argument_passing().pass_argument(&parameter, &value);
        assert_eq!(passed.expect("the argument passes"), Borrow::Pending(0));
        assert_eq!(fixture.deferred().borrows.len(), 1);
    }

    #[test]
    fn a_value_passes_borrowed_to_a_ref_parameter() {
        let mut fixture = PassingFixture::new();
        let parameter = Type::shared(PassingFixture::int());
        let value = fixture.call_result(PassingFixture::int());
        let passed = fixture.argument_passing().pass_argument(&parameter, &value);
        assert_eq!(passed.expect("the argument passes"), Borrow::Shared);
    }

    #[test]
    fn a_call_result_cannot_pass_to_a_mut_parameter() {
        let mut fixture = PassingFixture::new();
        let parameter = Type::mutable(PassingFixture::int());
        let value = fixture.call_result(PassingFixture::int());
        let passed = fixture.argument_passing().pass_argument(&parameter, &value);
        let diagnostic = passed.expect_err("a copy is no place");
        assert_eq!(
            diagnostic.title(),
            "this argument is a copy, so changing it changes nothing"
        );
    }

    #[test]
    fn a_local_passes_mutably_to_a_mut_parameter() {
        let mut fixture = PassingFixture::new();
        let parameter = Type::mutable(PassingFixture::int());
        let value = fixture.local(PassingFixture::int());
        let passed = fixture.argument_passing().pass_argument(&parameter, &value);
        assert_eq!(passed.expect("the argument passes"), Borrow::Mutable);
    }
}

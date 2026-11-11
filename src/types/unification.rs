use super::declarations::Declarations;
use super::display::TypeDisplay;
use super::expectation::Expectation;
use super::peeling::ReferencePeeling;
use super::problems::TypeProblems;
use super::ty::Type;
use super::typed::{TypedExpr, TypedExprKind, TypedStructExpr};
use super::unifier::{LiteralKind, Unifier, UnifyError};
use super::view_coercion::ViewCoercion;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub(super) struct Unification<'a> {
    unifier: &'a mut Unifier,
    display: TypeDisplay<'a>,
    declarations: Declarations<'a>,
}

impl<'a> Unification<'a> {
    pub(super) fn new(
        unifier: &'a mut Unifier,
        display: TypeDisplay<'a>,
        declarations: Declarations<'a>,
    ) -> Self {
        Self {
            unifier,
            display,
            declarations,
        }
    }

    pub(super) fn expect(
        &mut self,
        expectation: Expectation,
        found: &Type,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if self.coerces(expectation.expected, found, span) {
            return Ok(());
        }
        match self.unifier.unify(expectation.expected, found, span) {
            Ok(()) => Ok(()),
            Err(UnifyError::Infinite) => Err(TypeProblems::infinite(span)),
            Err(_) => Err(self.mismatch(expectation, found, span)),
        }
    }

    pub(super) fn coerces(&mut self, expected: &Type, found: &Type, span: Span) -> bool {
        match (self.unifier.resolve(expected), self.unifier.resolve(found)) {
            (Type::Ref(_, shared), Type::Ref(_, owned)) => {
                ViewCoercion::new(&shared, &owned).views_string()
                    || self.coerces(&shared, &owned, span)
            }
            (Type::Ref(_, shared), Type::Mut(_, mutable)) => {
                self.unifier.unify(&shared, &mutable, span).is_ok()
            }
            (Type::Mut(_, slice), Type::Mut(_, vector)) => self.coerces(&slice, &vector, span),
            (view, owned) => self.unifies_viewed_items(ViewCoercion::new(&view, &owned), span),
        }
    }

    pub(super) fn describe(&self, described: &Type) -> String {
        match described {
            Type::Var(var) => match self.unifier.literal_kind(*var) {
                Some(LiteralKind::Integer) => "an integer".to_string(),
                Some(LiteralKind::Float) => "a float".to_string(),
                None => "`_`".to_string(),
            },
            other => format!("`{}`", self.display.show(other)),
        }
    }

    pub(super) fn place_type(&self, value: &TypedExpr) -> Type {
        let TypedExprKind::Struct(TypedStructExpr::Field(owner, label)) = &value.kind else {
            return value.ty.clone();
        };
        let (owner, _) = ReferencePeeling::new(self.unifier).peel(&owner.ty);
        let Type::Named(binding, arguments) = owner else {
            return value.ty.clone();
        };
        self.declarations
            .field_type(binding, &label.name, &arguments)
            .unwrap_or_else(|| value.ty.clone())
    }

    fn unifies_viewed_items(&mut self, coercion: ViewCoercion, span: Span) -> bool {
        let Some(items) = self.declarations.viewed_items(coercion) else {
            return false;
        };
        items
            .iter()
            .all(|(item, element)| self.unifier.unify(item, element, span).is_ok())
    }

    fn mismatch(&self, expectation: Expectation, found: &Type, span: Span) -> Diagnostic {
        let expected = self.unifier.resolve(expectation.expected);
        let found = self.unifier.resolve(found);
        match &expected {
            Type::Generic(_) => TypeProblems::generic_mismatch(
                &self.display.show(&expected),
                &self.describe(&found),
                span,
            ),
            _ => TypeProblems::mismatch(
                &self.describe(&expected),
                &self.describe(&found),
                span,
                expectation.context,
            ),
        }
    }
}

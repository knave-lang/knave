use super::{ChildTyping, ExpressionForm, Obligations, TypeFitting, Unknowns};
use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::expectation::Expectation;
use crate::types::ty::Type;
use crate::types::typed::{TypedExpr, TypedExprKind};

pub(in crate::types) struct Dereference<'e> {
    value: &'e Expr,
    span: Span,
}

impl<'e> Dereference<'e> {
    pub(in crate::types) fn new(value: &'e Expr, span: Span) -> Self {
        Self { value, span }
    }
}

impl<C> ExpressionForm<C> for Dereference<'_>
where
    C: ChildTyping + TypeFitting + Unknowns + Obligations,
{
    fn type_against(
        &self,
        context: &mut C,
        _hint: Option<Expectation>,
    ) -> Result<TypedExpr, Diagnostic> {
        let value = context.type_child(self.value)?;
        let referent = context.fresh();
        context.require_copy(&referent);
        if let Type::Mut(_, inner) = context.resolve(&value.ty) {
            context.fit(Expectation::plain(&referent), &inner, value.span)?;
        } else {
            let borrowed = Type::shared(referent.clone());
            context.fit(Expectation::plain(&borrowed), &value.ty, value.span)?;
        }
        Ok(TypedExpr {
            kind: TypedExprKind::Deref(Box::new(value)),
            ty: referent,
            span: self.span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Dereference;
    use crate::core::{Expr, Literal, PrimitiveType};
    use crate::diagnostic::Diagnostic;
    use crate::source::Span;
    use crate::types::expectation::Expectation;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{ChildTyping, ExpressionForm, Obligations, TypeFitting, Unknowns};
    use crate::types::ty::Type;
    use crate::types::typed::{TypedExpr, TypedExprKind, TypedLeaf};

    struct DereferenceFixture {
        child_type: Type,
        fits: Vec<(Type, Type)>,
        copies: Vec<Type>,
    }

    impl DereferenceFixture {
        fn of(child_type: Type) -> Self {
            Self {
                child_type,
                fits: Vec::new(),
                copies: Vec::new(),
            }
        }

        fn dereference(&mut self) -> TypedExpr {
            let value = FormFixtures::leaf(Literal::Bool(true));
            Dereference::new(&value, FormFixtures::span())
                .type_against(self, None)
                .unwrap_or_else(|_| panic!("the fixture fits every type"))
        }

        fn referent() -> Type {
            Type::Primitive(PrimitiveType::U8)
        }

        fn inner() -> Type {
            Type::Primitive(PrimitiveType::I64)
        }
    }

    impl ChildTyping for DereferenceFixture {
        fn type_child(&mut self, child: &Expr) -> Result<TypedExpr, Diagnostic> {
            Ok(TypedExpr {
                kind: TypedExprKind::Leaf(TypedLeaf::Literal(Literal::Bool(true))),
                ty: self.child_type.clone(),
                span: child.span(),
            })
        }
    }

    impl TypeFitting for DereferenceFixture {
        fn fit(
            &mut self,
            expectation: Expectation,
            found: &Type,
            _span: Span,
        ) -> Result<(), Diagnostic> {
            self.fits
                .push((expectation.expected.clone(), found.clone()));
            Ok(())
        }
    }

    impl Unknowns for DereferenceFixture {
        fn fresh(&mut self) -> Type {
            Self::referent()
        }

        fn resolve(&self, value: &Type) -> Type {
            value.clone()
        }
    }

    impl Obligations for DereferenceFixture {
        fn require_copy(&mut self, value: &Type) {
            self.copies.push(value.clone());
        }
    }

    #[test]
    fn a_mut_borrow_gives_its_inner_type_to_the_referent() {
        let mut fixture = DereferenceFixture::of(Type::mutable(DereferenceFixture::inner()));
        fixture.dereference();
        assert_eq!(
            fixture.fits,
            [(DereferenceFixture::referent(), DereferenceFixture::inner())]
        );
    }

    #[test]
    fn a_shared_borrow_is_fitted_to_a_borrow_of_the_referent() {
        let borrowed = Type::shared(DereferenceFixture::inner());
        let mut fixture = DereferenceFixture::of(borrowed.clone());
        fixture.dereference();
        assert_eq!(
            fixture.fits,
            [(Type::shared(DereferenceFixture::referent()), borrowed)]
        );
    }

    #[test]
    fn the_referent_must_be_copy_and_is_the_result() {
        let mut fixture = DereferenceFixture::of(Type::shared(DereferenceFixture::inner()));
        let typed = fixture.dereference();
        assert_eq!(
            (fixture.copies, typed.ty),
            (
                vec![DereferenceFixture::referent()],
                DereferenceFixture::referent()
            )
        );
    }
}

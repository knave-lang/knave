use super::WriteCollectionTyping;
use crate::diagnostic::Diagnostic;
use crate::types::collection::Collection;
use crate::types::problems::{MismatchContext, TypeProblems};
use crate::types::ty::Type;
use crate::types::typed::TypedExpr;

pub(in crate::types) struct WriteCollection<'e> {
    base: &'e TypedExpr,
    depth: usize,
}

impl<'e> WriteCollection<'e> {
    pub(in crate::types) fn new(base: &'e TypedExpr, depth: usize) -> Self {
        Self { base, depth }
    }

    pub(in crate::types) fn resolve<C: WriteCollectionTyping>(
        &self,
        context: &mut C,
    ) -> Result<Collection, Diagnostic> {
        let resolved = context.resolve(&self.base.ty);
        let collection = match resolved {
            Type::Var(var) if context.literal_kind(var).is_none() => self.unknown_vector(context),
            _ => context.collection_of_referent(&resolved),
        };
        collection.ok_or_else(|| {
            TypeProblems::mismatch(
                "a collection",
                &context.show(&resolved),
                self.base.span,
                MismatchContext::Plain,
            )
        })
    }

    fn unknown_vector<C: WriteCollectionTyping>(&self, context: &mut C) -> Option<Collection> {
        let element = context.fresh();
        let vector = context.nested_vector_of(self.depth, &element);
        context.unify(&self.base.ty, &vector, self.base.span).ok()?;
        context.collection_of(&vector)
    }
}

#[cfg(test)]
mod tests {
    use super::WriteCollection;
    use crate::core::Literal;
    use crate::diagnostic::Diagnostic;
    use crate::source::Span;
    use crate::types::collection::Collection;
    use crate::types::forms::collection_fixtures::CollectionFixtures;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{
        CollectionShapes, LiteralKinds, ReferentCollections, TypeDisplays, TypeUnification,
        Unknowns,
    };
    use crate::types::ty::{Type, TypeVar};
    use crate::types::unifier::{LiteralKind, UnifyError};

    struct CollectionFixture {
        collections: CollectionFixtures,
        literal: Option<LiteralKind>,
        unified: Vec<Type>,
    }

    impl CollectionFixture {
        fn new() -> Self {
            Self {
                collections: CollectionFixtures::new(),
                literal: None,
                unified: Vec::new(),
            }
        }

        fn unknown() -> Type {
            Type::Var(TypeVar::new(0))
        }

        fn resolve_base(&mut self, base: Type, depth: usize) -> Result<Collection, Diagnostic> {
            let typed = FormFixtures::typed_leaf(Literal::Bool(true), base);
            WriteCollection::new(&typed, depth).resolve(self)
        }
    }

    impl Unknowns for CollectionFixture {
        fn fresh(&mut self) -> Type {
            CollectionFixtures::element()
        }

        fn resolve(&self, value: &Type) -> Type {
            value.clone()
        }
    }

    impl LiteralKinds for CollectionFixture {
        fn literal_kind(&self, _var: TypeVar) -> Option<LiteralKind> {
            self.literal
        }
    }

    impl TypeUnification for CollectionFixture {
        fn unify(&mut self, _left: &Type, right: &Type, _span: Span) -> Result<(), UnifyError> {
            self.unified.push(right.clone());
            Ok(())
        }
    }

    impl CollectionShapes for CollectionFixture {
        fn collection_of(&self, ty: &Type) -> Option<Collection> {
            self.collections.collection_of(ty)
        }

        fn nested_vector_of(&self, depth: usize, element: &Type) -> Type {
            CollectionFixtures::nested_vector_of(depth, element)
        }
    }

    impl ReferentCollections for CollectionFixture {
        fn collection_of_referent(&self, ty: &Type) -> Option<Collection> {
            self.collections.collection_of(ty)
        }
    }

    impl TypeDisplays for CollectionFixture {
        fn show(&self, _ty: &Type) -> String {
            "shown".to_string()
        }
    }

    #[test]
    fn an_unknown_base_becomes_a_vector_as_deep_as_the_write() {
        let mut fixture = CollectionFixture::new();
        let _ = fixture.resolve_base(CollectionFixture::unknown(), 2);
        let expected = CollectionFixtures::nested_vector_of(2, &CollectionFixtures::element());
        assert_eq!(fixture.unified, [expected]);
    }

    #[test]
    fn an_unknown_literal_base_is_no_collection() {
        let mut fixture = CollectionFixture::new();
        fixture.literal = Some(LiteralKind::Integer);
        assert!(
            fixture
                .resolve_base(CollectionFixture::unknown(), 1)
                .is_err()
        );
    }

    #[test]
    fn a_borrowed_base_is_read_as_its_collection() {
        let base = Type::shared(CollectionFixtures::map());
        let collection = CollectionFixture::new().resolve_base(base, 1);
        assert!(collection.is_ok_and(|collection| collection.is_map()));
    }
}

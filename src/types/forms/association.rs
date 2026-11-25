use super::{AssociationTyping, ExpressionForm};
use crate::core::KeyedPath;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::collection::Collection;
use crate::types::expectation::Expectation;
use crate::types::problems::TypeProblems;
use crate::types::ty::Type;
use crate::types::typed::{TypedExpr, TypedExprKind};

pub(in crate::types) struct Association<'e> {
    path: &'e KeyedPath,
    span: Span,
}

impl<'e> Association<'e> {
    pub(in crate::types) fn new(path: &'e KeyedPath, span: Span) -> Self {
        Self { path, span }
    }

    fn type_map<C: AssociationTyping>(
        &self,
        context: &mut C,
        value: TypedExpr,
        map: Collection,
    ) -> Result<TypedExpr, Diagnostic> {
        let [key] = self.path.keys.as_slice() else {
            let found = self.path.keys.len();
            return Err(TypeProblems::wrong_argument_count(1, found, self.span));
        };
        let key = context.type_child(key)?;
        context.fit(Expectation::plain(map.key()), &key.ty, key.span)?;
        context.require_map_key(&map, key.span);
        let item = context.type_child(&self.path.item)?;
        context.fit(Expectation::plain(map.value()), &item.ty, item.span)?;
        Ok(TypedExpr {
            kind: TypedExprKind::Assoc(Box::new(value), vec![key], Box::new(item)),
            ty: map.ty().clone(),
            span: self.span,
        })
    }

    fn type_vector<C: AssociationTyping>(
        &self,
        context: &mut C,
        value: TypedExpr,
    ) -> Result<TypedExpr, Diagnostic> {
        let element = self.element(context, &value)?;
        let keys = self.type_keys(context)?;
        let item = context.type_child(&self.path.item)?;
        context.fit(Expectation::plain(&element), &item.ty, item.span)?;
        let ty = context.resolve(&value.ty);
        Ok(TypedExpr {
            kind: TypedExprKind::Assoc(Box::new(value), keys, Box::new(item)),
            ty,
            span: self.span,
        })
    }

    fn element<C: AssociationTyping>(
        &self,
        context: &mut C,
        base: &TypedExpr,
    ) -> Result<Type, Diagnostic> {
        let element = context.fresh();
        let expected = context.nested_vector_of(self.path.keys.len(), &element);
        let fitted = context.fit(Expectation::plain(&expected), &base.ty, base.span);
        fitted.map_err(|error| context.explain_struct_base(error, &base.ty))?;
        Ok(element)
    }

    fn type_keys<C: AssociationTyping>(
        &self,
        context: &mut C,
    ) -> Result<Vec<TypedExpr>, Diagnostic> {
        let mut typed_keys = Vec::new();
        for key in &self.path.keys {
            let typed = context.type_child(key)?;
            context.fit(
                Expectation::plain(&Collection::INDEX),
                &typed.ty,
                typed.span,
            )?;
            typed_keys.push(typed);
        }
        Ok(typed_keys)
    }
}

impl<C: AssociationTyping> ExpressionForm<C> for Association<'_> {
    fn type_against(
        &self,
        context: &mut C,
        _hint: Option<Expectation>,
    ) -> Result<TypedExpr, Diagnostic> {
        let value = context.type_child(&self.path.value)?;
        let value = context.moved_base(value);
        let resolved = context.resolve(&value.ty);
        match context.collection_of(&resolved).filter(Collection::is_map) {
            Some(map) => self.type_map(context, value, map),
            None => self.type_vector(context, value),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::Association;
    use crate::core::{Expr, ExprKind, KeyedPath, Leaf, Literal};
    use crate::diagnostic::Diagnostic;
    use crate::source::Span;
    use crate::types::collection::Collection;
    use crate::types::expectation::Expectation;
    use crate::types::forms::collection_fixtures::CollectionFixtures;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{
        ChildTyping, CollectionLookups, ExpressionForm, FieldMoves, MapKeys, PreludeShapes,
        StructBases, TypeFitting, Unknowns,
    };
    use crate::types::problems::TypeProblems;
    use crate::types::ty::Type;
    use crate::types::typed::TypedExpr;

    struct AssociationFixture {
        base: Type,
        collections: CollectionFixtures,
        refuses_base: bool,
        fitted: Vec<Type>,
        required_keys: usize,
        moved: bool,
    }

    impl AssociationFixture {
        fn of(base: Type) -> Self {
            Self {
                base,
                collections: CollectionFixtures::new(),
                refuses_base: false,
                fitted: Vec::new(),
                required_keys: 0,
                moved: false,
            }
        }

        fn associate(&mut self, keys: usize) -> Result<TypedExpr, Diagnostic> {
            let path = KeyedPath {
                value: Box::new(FormFixtures::leaf(Literal::String(Rc::from("base")))),
                keys: (0..keys).map(|_| Self::truth()).collect(),
                item: Box::new(Self::truth()),
            };
            Association::new(&path, FormFixtures::span()).type_against(self, None)
        }

        fn truth() -> Expr {
            FormFixtures::leaf(Literal::Bool(true))
        }
    }

    impl ChildTyping for AssociationFixture {
        fn type_child(&mut self, child: &Expr) -> Result<TypedExpr, Diagnostic> {
            let ty = match child.kind() {
                ExprKind::Leaf(Leaf::Literal(Literal::String(_))) => self.base.clone(),
                _ => Type::bool(),
            };
            Ok(FormFixtures::typed_leaf(Literal::Bool(true), ty))
        }
    }

    impl TypeFitting for AssociationFixture {
        fn fit(
            &mut self,
            expectation: Expectation,
            _found: &Type,
            span: Span,
        ) -> Result<(), Diagnostic> {
            self.fitted.push(expectation.expected.clone());
            if self.refuses_base && self.fitted.len() == 1 {
                return Err(TypeProblems::wrong_argument_count(1, 0, span));
            }
            Ok(())
        }
    }

    impl Unknowns for AssociationFixture {
        fn fresh(&mut self) -> Type {
            CollectionFixtures::element()
        }

        fn resolve(&self, value: &Type) -> Type {
            value.clone()
        }

        fn is_open(&self, _value: &Type) -> bool {
            false
        }
    }

    impl CollectionLookups for AssociationFixture {
        fn collection_of(&self, ty: &Type) -> Option<Collection> {
            self.collections.collection_of(ty)
        }

        fn collection_of_referent(&self, ty: &Type) -> Option<Collection> {
            self.collections.collection_of_referent(ty)
        }
    }

    impl PreludeShapes for AssociationFixture {
        fn nested_vector_of(&self, depth: usize, element: &Type) -> Type {
            CollectionFixtures::nested_vector_of(depth, element)
        }

        fn slice_of(&self, element: Type) -> Type {
            CollectionFixtures::slice_of(element)
        }

        fn option_of(&self, _value: Type) -> Type {
            unreachable!("this form builds no option")
        }
    }

    impl MapKeys for AssociationFixture {
        fn require_map_key(&mut self, _map: &Collection, _span: Span) {
            self.required_keys += 1;
        }

        fn require_key_borrow(&mut self, _map: &Collection, _query: &Type, _span: Span) {}
    }

    impl StructBases for AssociationFixture {
        fn explain_struct_base(&self, error: Diagnostic, _base: &Type) -> Diagnostic {
            error.with_help(TypeProblems::struct_base_help())
        }
    }

    impl FieldMoves for AssociationFixture {
        fn moved_base(&mut self, base: TypedExpr) -> TypedExpr {
            self.moved = true;
            base
        }
    }

    #[test]
    fn a_map_assoc_fits_its_key_and_item_to_the_map() {
        let mut fixture = AssociationFixture::of(CollectionFixtures::map());
        let _ = fixture.associate(1);
        assert_eq!(fixture.fitted, [CollectionFixtures::key(), Type::bool()]);
    }

    #[test]
    fn a_map_assoc_requires_its_key_bounds() {
        let mut fixture = AssociationFixture::of(CollectionFixtures::map());
        let _ = fixture.associate(1);
        assert_eq!(fixture.required_keys, 1);
    }

    #[test]
    fn a_map_assoc_takes_one_key() {
        let typed = AssociationFixture::of(CollectionFixtures::map()).associate(2);
        assert!(typed.is_err());
    }

    #[test]
    fn a_map_assoc_has_the_map_type() {
        let typed = AssociationFixture::of(CollectionFixtures::map()).associate(1);
        assert_eq!(
            typed.ok().map(|typed| typed.ty),
            Some(CollectionFixtures::map())
        );
    }

    #[test]
    fn an_assoc_reads_its_base_as_a_moved_field() {
        let mut fixture = AssociationFixture::of(CollectionFixtures::vector_of(Type::bool()));
        let _ = fixture.associate(1);
        assert!(fixture.moved);
    }

    #[test]
    fn a_vector_assoc_fits_its_base_to_a_vector_as_deep_as_its_keys() {
        let mut fixture = AssociationFixture::of(CollectionFixtures::vector_of(Type::bool()));
        let _ = fixture.associate(2);
        let expected = fixture.nested_vector_of(2, &CollectionFixtures::element());
        assert_eq!(fixture.fitted.first(), Some(&expected));
    }

    #[test]
    fn a_vector_assoc_fits_each_key_to_an_index() {
        let mut fixture = AssociationFixture::of(CollectionFixtures::vector_of(Type::bool()));
        let _ = fixture.associate(2);
        assert_eq!(fixture.fitted[1..3], [Collection::INDEX, Collection::INDEX]);
    }

    #[test]
    fn a_vector_assoc_fits_its_item_to_the_element() {
        let mut fixture = AssociationFixture::of(CollectionFixtures::vector_of(Type::bool()));
        let _ = fixture.associate(1);
        assert_eq!(fixture.fitted.last(), Some(&CollectionFixtures::element()));
    }

    #[test]
    fn a_vector_assoc_has_its_base_type() {
        let typed =
            AssociationFixture::of(CollectionFixtures::vector_of(Type::bool())).associate(1);
        assert_eq!(
            typed.ok().map(|typed| typed.ty),
            Some(CollectionFixtures::vector_of(Type::bool()))
        );
    }

    #[test]
    fn a_refused_base_explains_a_struct_base() {
        let mut fixture = AssociationFixture::of(CollectionFixtures::vector_of(Type::bool()));
        fixture.refuses_base = true;
        let error = fixture.associate(1).err();
        assert!(error.is_some_and(|error| !error.helps().is_empty()));
    }
}

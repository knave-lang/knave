use super::ty::Type;
use super::typed::{Borrow, TypedArgument};

impl TypedArgument {
    pub fn collapsed_shared_layers(&self) -> Option<usize> {
        let Borrow::Collapse(levels) = self.borrow else {
            return None;
        };
        let mut layer = &self.value.ty;
        for _ in 0..levels {
            let Type::Ref(_, inner) = layer else {
                return None;
            };
            layer = inner;
        }
        matches!(layer, Type::Ref(..)).then_some(levels)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::Literal;
    use crate::source::{SourceId, Span};
    use crate::types::{Borrow, Type, TypedArgument, TypedExpr, TypedExprKind, TypedLeaf};

    struct ArgumentFixture;

    impl ArgumentFixture {
        fn collapsed(ty: Type, levels: usize) -> Option<usize> {
            let value = TypedExpr {
                kind: TypedExprKind::Leaf(TypedLeaf::Literal(Literal::Bool(true))),
                ty,
                span: Span::new(SourceId::new(0), 0..0),
            };
            let borrow = Borrow::Collapse(levels);
            TypedArgument { value, borrow }.collapsed_shared_layers()
        }
    }

    #[test]
    fn a_collapse_through_shared_references_strips_its_layers() {
        let text = Type::str_ref();
        assert_eq!(
            ArgumentFixture::collapsed(Type::shared(text.clone()), 1),
            Some(1)
        );
        let twice = Type::shared(Type::shared(text));
        assert_eq!(ArgumentFixture::collapsed(twice, 2), Some(2));
    }

    #[test]
    fn a_collapse_through_a_mutable_layer_strips_nothing() {
        let mutable_text = Type::shared(Type::mutable(Type::str_ref()));
        assert_eq!(ArgumentFixture::collapsed(mutable_text.clone(), 2), None);
        assert_eq!(ArgumentFixture::collapsed(mutable_text, 1), None);
        let shared_text = Type::mutable(Type::shared(Type::str_ref()));
        assert_eq!(ArgumentFixture::collapsed(shared_text, 1), None);
    }
}

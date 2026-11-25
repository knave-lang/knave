use super::owned_place::OwnedPlace;
use super::typed::{AssignmentPath, TypedArgument, TypedAssignment, TypedExpr};
use super::typed::{TypedExprKind, TypedLeaf};
use crate::core::BindingId;

pub struct WholeWrite<'a> {
    place: &'a TypedArgument,
}

impl<'a> WholeWrite<'a> {
    pub fn of(assignment: &'a TypedAssignment) -> Option<Self> {
        (assignment.path == AssignmentPath::Whole).then(|| Self {
            place: assignment.place(),
        })
    }

    pub fn local(&self) -> Option<BindingId> {
        Self::local_of(&self.place.value)
    }

    pub fn replaced_local(&self) -> Option<BindingId> {
        OwnedPlace::of(self.place)?.local()
    }

    fn local_of(place: &TypedExpr) -> Option<BindingId> {
        match place.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(local)) => Some(local),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WholeWrite;
    use crate::core::{BindingId, Literal, ReferenceKind};
    use crate::source::{SourceId, Span};
    use crate::types::ClosureKind;
    use crate::types::rust_leaf::RustLeaf;
    use crate::types::synthetic_template::SyntheticTemplate;
    use crate::types::ty::Type;
    use crate::types::typed::{AssignmentPath, Borrow, TypedArgument, TypedAssignment};
    use crate::types::typed::{TypedExpr, TypedExprKind, TypedLeaf};

    struct WholeFixture;

    impl WholeFixture {
        const LOCAL: BindingId = BindingId::from_index(7);

        fn local(ty: Type) -> TypedExpr {
            TypedExpr {
                kind: TypedExprKind::Leaf(TypedLeaf::Local(Self::LOCAL)),
                ty,
                span: WholeFixture::span(),
            }
        }

        fn written_mut(ty: Type) -> TypedExpr {
            let place = Self::local(ty.clone());
            TypedExpr {
                kind: TypedExprKind::Borrow(ReferenceKind::Mutable, Box::new(place)),
                ty: Type::mutable(ty),
                span: WholeFixture::span(),
            }
        }

        fn span() -> Span {
            Span::new(SourceId::new(0), 0..1)
        }

        fn item() -> TypedArgument {
            TypedArgument {
                value: TypedExpr {
                    kind: TypedExprKind::Leaf(TypedLeaf::Literal(Literal::Bool(true))),
                    ty: Type::bool(),
                    span: Self::span(),
                },
                borrow: Borrow::None,
            }
        }

        fn assignment(path: AssignmentPath, place: TypedExpr, borrow: Borrow) -> TypedAssignment {
            let signature = Type::Function(ClosureKind::Fn, Vec::new(), Box::new(Type::unit()));
            let template =
                RustLeaf::new(SyntheticTemplate::ASSIGN.text()).typed(signature, Self::span());
            let place = TypedArgument {
                value: place,
                borrow,
            };
            TypedAssignment::new(path, template, vec![place, WholeFixture::item()])
        }

        fn replaced(place: TypedExpr, borrow: Borrow) -> Option<BindingId> {
            WholeWrite::of(&Self::assignment(AssignmentPath::Whole, place, borrow))?
                .replaced_local()
        }
    }

    #[test]
    fn an_owned_local_assigned_whole_is_replaced() {
        let place = WholeFixture::local(Type::bool());
        let replaced = WholeFixture::replaced(place, Borrow::Mutable);
        assert_eq!(replaced, Some(WholeFixture::LOCAL));
    }

    #[test]
    fn a_written_mut_of_an_owned_local_is_replaced() {
        let place = WholeFixture::written_mut(Type::bool());
        let replaced = WholeFixture::replaced(place, Borrow::None);
        assert_eq!(replaced, Some(WholeFixture::LOCAL));
    }

    #[test]
    fn a_local_reached_through_a_mut_reference_is_written_through_it() {
        let place = WholeFixture::local(Type::mutable(Type::bool()));
        assert_eq!(WholeFixture::replaced(place, Borrow::Mutable), None);
    }

    #[test]
    fn a_reborrowed_local_is_written_through_its_reference() {
        let place = WholeFixture::local(Type::bool());
        assert_eq!(WholeFixture::replaced(place, Borrow::Reborrow), None);
    }

    #[test]
    fn a_written_mut_of_a_reference_is_written_through_it() {
        let place = WholeFixture::written_mut(Type::mutable(Type::bool()));
        assert_eq!(WholeFixture::replaced(place, Borrow::None), None);
    }

    #[test]
    fn a_keyed_write_replaces_no_local() {
        let place = WholeFixture::local(Type::bool());
        let assignment = WholeFixture::assignment(AssignmentPath::Elements, place, Borrow::Mutable);
        assert!(WholeWrite::of(&assignment).is_none());
    }

    #[test]
    fn a_whole_write_of_a_literal_replaces_no_local() {
        let place = WholeFixture::item().value;
        assert_eq!(WholeFixture::replaced(place, Borrow::Mutable), None);
    }

    #[test]
    fn a_whole_write_names_its_local_even_through_a_reference() {
        let place = WholeFixture::local(Type::mutable(Type::bool()));
        let assignment = WholeFixture::assignment(AssignmentPath::Whole, place, Borrow::Mutable);
        let local = WholeWrite::of(&assignment).and_then(|write| write.local());
        assert_eq!(local, Some(WholeFixture::LOCAL));
    }
}

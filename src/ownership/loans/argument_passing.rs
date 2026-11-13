use super::loan_context::LoanContext;
use crate::core::ReferenceKind;
use crate::types::{Borrow, Type, TypedArgument};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgumentPassing {
    Lends(ReferenceKind),
    Moves,
    Copies,
    Holds,
}

impl ArgumentPassing {
    pub fn of(
        argument: &TypedArgument,
        parameter: Option<&Type>,
        context: &LoanContext<'_>,
    ) -> Self {
        match argument.borrow {
            Borrow::Shared => Self::Lends(ReferenceKind::Shared),
            Borrow::Mutable | Borrow::Reborrow => Self::Lends(ReferenceKind::Mutable),
            Borrow::Deref(_) => Self::Copies,
            Borrow::Passthrough | Borrow::Collapse(_) if argument.value.ty.is_reference() => {
                Self::through_reference(parameter)
            }
            Borrow::None if context.moves(&argument.value) => Self::Moves,
            _ => Self::Holds,
        }
    }

    pub fn takes_the_place(self) -> bool {
        matches!(self, Self::Lends(_) | Self::Moves)
    }

    fn through_reference(parameter: Option<&Type>) -> Self {
        match parameter {
            Some(Type::Mut(..)) => Self::Lends(ReferenceKind::Mutable),
            Some(Type::Ref(..)) => Self::Lends(ReferenceKind::Shared),
            _ => Self::Holds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ArgumentPassing;
    use crate::core::ReferenceKind;
    use crate::ownership::lambda_source::LambdaSource;
    use crate::ownership::loans::loan_context::LoanContext;
    use crate::types::{Borrow, Type, TypedArgument, TypedExpr, TypedExprKind, TypedLeaf};

    struct PassingFixture {
        source: LambdaSource,
    }

    impl PassingFixture {
        fn new() -> Self {
            Self {
                source: LambdaSource::new(
                    "(defn main ()\n  (local v (vec 1))\n  (local n 1)\n  (local r (ref v))\n  (println n (len v) (len r)))",
                ),
            }
        }

        fn passing(
            &self,
            local: &str,
            borrow: Borrow,
            parameter: Option<&Type>,
        ) -> ArgumentPassing {
            let binder = self.source.binder(local);
            let argument = TypedArgument {
                value: TypedExpr {
                    kind: TypedExprKind::Leaf(TypedLeaf::Local(binder.binding)),
                    ty: binder.ty.clone(),
                    span: binder.span,
                },
                borrow,
            };
            ArgumentPassing::of(
                &argument,
                parameter,
                &LoanContext::of(self.source.program()),
            )
        }
    }

    #[test]
    fn a_borrow_adjustment_lends_and_a_dereference_copies() {
        let fixture = PassingFixture::new();
        assert_eq!(
            fixture.passing("v", Borrow::Shared, None),
            ArgumentPassing::Lends(ReferenceKind::Shared)
        );
        assert_eq!(
            fixture.passing("v", Borrow::Reborrow, None),
            ArgumentPassing::Lends(ReferenceKind::Mutable)
        );
        assert_eq!(
            fixture.passing("r", Borrow::Deref(1), None),
            ArgumentPassing::Copies
        );
    }

    #[test]
    fn a_reference_passed_through_lends_as_its_parameter_takes_it() {
        let fixture = PassingFixture::new();
        let changing = Type::mutable(Type::unit());
        let reading = Type::shared(Type::unit());
        assert_eq!(
            fixture.passing("r", Borrow::Passthrough, Some(&changing)),
            ArgumentPassing::Lends(ReferenceKind::Mutable)
        );
        assert_eq!(
            fixture.passing("r", Borrow::Passthrough, Some(&reading)),
            ArgumentPassing::Lends(ReferenceKind::Shared)
        );
        assert_eq!(
            fixture.passing("r", Borrow::Passthrough, None),
            ArgumentPassing::Holds
        );
    }

    #[test]
    fn an_unadjusted_value_moves_unless_it_is_copy() {
        let fixture = PassingFixture::new();
        assert_eq!(
            fixture.passing("v", Borrow::None, None),
            ArgumentPassing::Moves
        );
        assert_eq!(
            fixture.passing("n", Borrow::None, None),
            ArgumentPassing::Holds
        );
        assert!(ArgumentPassing::Moves.takes_the_place());
        assert!(!ArgumentPassing::Copies.takes_the_place());
    }
}

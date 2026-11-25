use super::PairPartTyping;
use crate::core::Expr;
use crate::diagnostic::Diagnostic;
use crate::types::positional_read::PositionalRead;
use crate::types::typed::{TypedArgument, TypedExpr, TypedExprKind};

pub(in crate::types) struct PairPartRead<'e> {
    read: PositionalRead,
    arguments: &'e [Expr],
}

impl<'e> PairPartRead<'e> {
    const READER_ARGUMENTS: usize = 1;

    pub(in crate::types) fn new(read: PositionalRead, arguments: &'e [Expr]) -> Self {
        Self { read, arguments }
    }

    pub(in crate::types) fn type_with<C: PairPartTyping>(
        &self,
        context: &mut C,
        pair: TypedExpr,
    ) -> Result<TypedExpr, Diagnostic> {
        let reader = self.read.reader(self.arguments)?;
        if let Some(part) = context.pair_part(&self.read, &pair.ty) {
            return Ok(context.read_part(part, pair));
        }
        context.assume_unknown_collection(&pair, self.read.slice_assumption());
        let callee = context.type_child(reader)?;
        let (parameters, ty) = context.function_shape(&callee, Self::READER_ARGUMENTS)?;
        let borrow = context.pass_first_argument(&callee, &parameters[0], &pair)?;
        let argument = TypedArgument {
            value: pair,
            borrow,
        };
        let kind = TypedExprKind::Call(Box::new(callee), vec![argument]);
        Ok(TypedExpr {
            kind,
            ty,
            span: self.read.span(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::PairPartRead;
    use crate::core::{Expr, Literal, PairPosition};
    use crate::diagnostic::Diagnostic;
    use crate::types::assumed_collections::CollectionAssumption;
    use crate::types::forms::collection_fixtures::CollectionFixtures;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{
        ArgumentTyping, CalleeShapes, ChildTyping, CollectionAssumptions, PairParts,
    };
    use crate::types::positional_read::{PairPart, PositionalRead};
    use crate::types::ty::Type;
    use crate::types::typed::{Borrow, TypedArgument, TypedExpr, TypedExprKind};

    #[derive(Default)]
    struct PartFixture {
        collections: CollectionFixtures,
        pair_known: bool,
        assumptions: Vec<CollectionAssumption>,
        readers_typed: usize,
    }

    impl PartFixture {
        fn read(&mut self, arguments: usize) -> Result<TypedExpr, Diagnostic> {
            let arguments: Vec<Expr> = (0..arguments)
                .map(|_| FormFixtures::leaf(Literal::Bool(true)))
                .collect();
            let read = PositionalRead::first(FormFixtures::span());
            let pair = FormFixtures::typed_leaf(Literal::Bool(true), Type::bool());
            PairPartRead::new(read, &arguments).type_with(self, pair)
        }

        fn reader_result() -> Type {
            Type::unit()
        }
    }

    impl PairParts for PartFixture {
        fn pair_part(&self, read: &PositionalRead, pair: &Type) -> Option<PairPart> {
            self.pair_known
                .then(|| self.collections.pair_part(read, pair))
                .flatten()
        }

        fn read_part(&self, part: PairPart, pair: TypedExpr) -> TypedExpr {
            part.read(pair, |_| true)
        }
    }

    impl CollectionAssumptions for PartFixture {
        fn assume_unknown_collection(
            &mut self,
            _target: &TypedExpr,
            assumption: CollectionAssumption,
        ) {
            self.assumptions.push(assumption);
        }
    }

    impl ChildTyping for PartFixture {
        fn type_child(&mut self, child: &Expr) -> Result<TypedExpr, Diagnostic> {
            self.readers_typed += 1;
            Ok(TypedExpr {
                kind: FormFixtures::typed_leaf(Literal::Bool(true), Type::bool()).kind,
                ty: Type::bool(),
                span: child.span(),
            })
        }
    }

    impl CalleeShapes for PartFixture {
        fn function_shape(
            &mut self,
            _callee: &TypedExpr,
            argument_count: usize,
        ) -> Result<(Vec<Type>, Type), Diagnostic> {
            let parameters = vec![Type::bool(); argument_count];
            Ok((parameters, Self::reader_result()))
        }
    }

    impl ArgumentTyping for PartFixture {
        fn type_arguments(
            &mut self,
            _callee: Option<&TypedExpr>,
            _parameters: &[Type],
            _arguments: &[Expr],
        ) -> Result<Vec<TypedArgument>, Diagnostic> {
            unreachable!("a pair part passes its pair, already typed")
        }

        fn pass_first_argument(
            &mut self,
            _callee: &TypedExpr,
            _parameter: &Type,
            _value: &TypedExpr,
        ) -> Result<Borrow, Diagnostic> {
            Ok(Borrow::Shared)
        }
    }

    #[test]
    fn a_pair_part_read_without_one_reader_is_refused() {
        assert!(PartFixture::default().read(2).is_err());
    }

    #[test]
    fn a_known_pair_reads_its_part_as_a_field() {
        let mut fixture = PartFixture {
            pair_known: true,
            ..Default::default()
        };
        let read = fixture.read(1);
        assert!(read.is_ok_and(|read| matches!(read.kind, TypedExprKind::Struct(_))));
        assert_eq!(fixture.readers_typed, 0);
    }

    #[test]
    fn an_unknown_pair_is_assumed_a_collection() {
        let mut fixture = PartFixture::default();
        let _ = fixture.read(1);
        assert!(matches!(
            fixture.assumptions.as_slice(),
            [CollectionAssumption::PairPartCollection(
                PairPosition::First
            )]
        ));
    }

    #[test]
    fn an_unknown_pair_is_passed_to_its_reader() {
        let read = PartFixture::default().read(1);
        let passed = read.ok().and_then(|read| match read.kind {
            TypedExprKind::Call(_, arguments) => arguments.first().map(|argument| argument.borrow),
            _ => None,
        });
        assert_eq!(passed, Some(Borrow::Shared));
    }

    #[test]
    fn an_unknown_pair_read_has_the_reader_result() {
        let ty = PartFixture::default().read(1).ok().map(|read| read.ty);
        assert_eq!(ty, Some(PartFixture::reader_result()));
    }
}

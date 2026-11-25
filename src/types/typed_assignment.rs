use super::typed::{AssignmentPath, TypedArgument, TypedAssignment, TypedExpr, TypedExprKind};

impl TypedAssignment {
    const PLACE_POSITION: usize = 0;

    pub fn written(call: TypedExpr, path: AssignmentPath) -> TypedExpr {
        let TypedExprKind::Call(template, arguments) = call.kind else {
            unreachable!("the checker types a place write as a synthetic call first")
        };
        let assignment = Self {
            path,
            template: *template,
            arguments,
        };
        TypedExpr {
            kind: TypedExprKind::Assign(Box::new(assignment)),
            ty: call.ty,
            span: call.span,
        }
    }

    pub fn place(&self) -> &TypedArgument {
        &self.arguments[Self::PLACE_POSITION]
    }

    pub fn keys(&self) -> &[TypedArgument] {
        &self.arguments[Self::PLACE_POSITION + 1..self.value_position()]
    }

    pub fn value(&self) -> &TypedArgument {
        &self.arguments[self.value_position()]
    }

    fn value_position(&self) -> usize {
        self.arguments.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use crate::core::Literal;
    use crate::source::{SourceId, Span};
    use crate::types::ClosureKind;
    use crate::types::rust_leaf::RustLeaf;
    use crate::types::ty::Type;
    use crate::types::typed::{AssignmentPath, TypedAssignment};
    use crate::types::typed::{Borrow, TypedArgument, TypedExpr, TypedExprKind, TypedLeaf};

    struct WriteFixture;

    impl WriteFixture {
        const TEMPLATE: &'static str = "{0}[{index 1}] = {2}";

        fn call() -> TypedExpr {
            let signature = Type::Function(ClosureKind::Fn, Vec::new(), Box::new(Type::unit()));
            let template = RustLeaf::new(Self::TEMPLATE).typed(signature, Self::span());
            let arguments = (0..3).map(Self::argument).collect();
            TypedExpr {
                kind: TypedExprKind::Call(Box::new(template), arguments),
                ty: Type::unit(),
                span: Self::span(),
            }
        }

        fn argument(position: usize) -> TypedArgument {
            TypedArgument {
                value: TypedExpr {
                    kind: TypedExprKind::Leaf(TypedLeaf::Literal(Literal::Bool(true))),
                    ty: Type::bool(),
                    span: Span::new(SourceId::new(0), position..position + 1),
                },
                borrow: Borrow::Mutable,
            }
        }

        fn starts(arguments: &[TypedArgument]) -> Vec<usize> {
            (arguments.iter())
                .map(|argument| argument.value.span.range().start)
                .collect()
        }

        fn span() -> Span {
            Span::new(SourceId::new(0), 3..9)
        }

        fn assignment(written: &TypedExpr) -> &TypedAssignment {
            match &written.kind {
                TypedExprKind::Assign(assignment) => assignment,
                _ => panic!("a written place is an assignment"),
            }
        }
    }

    #[test]
    fn a_written_place_keeps_the_call_template_and_arguments() {
        let written = TypedAssignment::written(WriteFixture::call(), AssignmentPath::Elements);
        let assignment = WriteFixture::assignment(&written);
        let TypedExprKind::Leaf(TypedLeaf::Rust(template)) = &assignment.template.kind else {
            panic!("the template is a Rust leaf");
        };
        assert_eq!(&*template.template, WriteFixture::TEMPLATE);
        assert_eq!(assignment.arguments.len(), 3);
        assert_eq!(assignment.path, AssignmentPath::Elements);
    }

    #[test]
    fn a_written_place_keeps_the_call_type_and_span() {
        let written = TypedAssignment::written(WriteFixture::call(), AssignmentPath::MapEntry);
        assert_eq!(written.ty, Type::unit());
        assert_eq!(written.span, WriteFixture::span());
    }

    #[test]
    fn a_written_place_names_its_place_keys_and_value() {
        let written = TypedAssignment::written(WriteFixture::call(), AssignmentPath::Elements);
        let assignment = WriteFixture::assignment(&written);
        assert_eq!(assignment.place().value.span.range().start, 0);
        assert_eq!(WriteFixture::starts(assignment.keys()), [1]);
        assert_eq!(assignment.value().value.span.range().start, 2);
    }
}

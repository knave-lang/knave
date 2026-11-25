use super::typed::{AssignmentPath, TypedArgument, TypedAssignment, TypedExpr};

impl TypedAssignment {
    const PLACE_POSITION: usize = 0;

    pub fn new(path: AssignmentPath, template: TypedExpr, arguments: Vec<TypedArgument>) -> Self {
        Self {
            path,
            template,
            arguments,
        }
    }

    pub fn place(&self) -> &TypedArgument {
        &self.arguments[Self::PLACE_POSITION]
    }

    pub fn keys(&self) -> &[TypedArgument] {
        &self.arguments[Self::PLACE_POSITION + 1..self.value_position()]
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

        fn assignment(path: AssignmentPath) -> TypedAssignment {
            let signature = Type::Function(ClosureKind::Fn, Vec::new(), Box::new(Type::unit()));
            let template = RustLeaf::new(Self::TEMPLATE).typed(signature, Self::span(0));
            let arguments = (0..3).map(Self::argument).collect();
            TypedAssignment::new(path, template, arguments)
        }

        fn argument(position: usize) -> TypedArgument {
            TypedArgument {
                value: TypedExpr {
                    kind: TypedExprKind::Leaf(TypedLeaf::Literal(Literal::Bool(true))),
                    ty: Type::bool(),
                    span: Self::span(position),
                },
                borrow: Borrow::Mutable,
            }
        }

        fn span(position: usize) -> Span {
            Span::new(SourceId::new(0), position..position + 1)
        }

        fn starts(arguments: &[TypedArgument]) -> Vec<usize> {
            (arguments.iter())
                .map(|argument| argument.value.span.range().start)
                .collect()
        }
    }

    #[test]
    fn an_assignment_keeps_its_path_and_template() {
        let assignment = WriteFixture::assignment(AssignmentPath::MapEntry);
        let TypedExprKind::Leaf(TypedLeaf::Rust(template)) = &assignment.template.kind else {
            panic!("the template is a Rust leaf");
        };
        assert_eq!(&*template.template, WriteFixture::TEMPLATE);
        assert_eq!(assignment.path, AssignmentPath::MapEntry);
    }

    #[test]
    fn an_assignment_names_its_place_and_keys() {
        let assignment = WriteFixture::assignment(AssignmentPath::Elements);
        assert_eq!(assignment.place().value.span.range().start, 0);
        assert_eq!(WriteFixture::starts(assignment.keys()), [1]);
    }
}

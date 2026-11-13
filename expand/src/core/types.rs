use super::{BindingId, PrimitiveType, ReferenceKind};
use crate::source::Span;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum ClosureKind {
    Fn,
    FnMut,
    FnOnce,
}

impl ClosureKind {
    pub fn meets(self, required: Self) -> bool {
        matches!(
            (self, required),
            (Self::Fn, _)
                | (Self::FnMut, Self::FnMut | Self::FnOnce)
                | (Self::FnOnce, Self::FnOnce)
        )
    }

    pub fn keyword(self) -> &'static str {
        match self {
            Self::Fn => "Fn",
            Self::FnMut => "FnMut",
            Self::FnOnce => "FnOnce",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Fn => "fn",
            Self::FnMut => "fn-mut",
            Self::FnOnce => "fn-once",
        }
    }

    pub fn shape(self) -> &'static str {
        match self {
            Self::Fn => "(fn (argument ...) result)",
            Self::FnMut => "(fn-mut (argument ...) result)",
            Self::FnOnce => "(fn-once (argument ...) result)",
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TypeExpr {
    kind: TypeExprKind,
    span: Span,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum TypeExprKind {
    Primitive(PrimitiveType),
    Named(BindingId, Vec<TypeExpr>),
    Function(ClosureKind, Vec<TypeExpr>, Box<TypeExpr>),
    Reference(ReferenceKind, Option<BindingId>, Box<TypeExpr>),
    Hole,
    Where(Vec<TypeBinder>, Box<TypeExpr>),
    With(Box<TypeExpr>, Box<TypeExpr>),
    Trait(BindingId),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TypeBinder {
    pub variable: BindingId,
    pub bounds: Vec<TypeBound>,
    pub span: Span,
    pub lifetime: bool,
}

impl TypeBinder {
    pub const UNSIZED: &'static str = "unsized";
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TypeBound {
    pub binding: BindingId,
    pub argument: Option<TypeExpr>,
}

impl TypeBound {
    pub fn named_bindings(&self) -> Vec<BindingId> {
        let mut named = Vec::new();
        self.collect_named(&mut named);
        named
    }

    fn collect_named(&self, named: &mut Vec<BindingId>) {
        named.push(self.binding);
        if let Some(argument) = &self.argument {
            argument.collect_named(named);
        }
    }
}

impl TypeExpr {
    pub fn new(kind: TypeExprKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn kind(&self) -> &TypeExprKind {
        &self.kind
    }

    pub fn parts(&self) -> Vec<&TypeExpr> {
        match self.kind() {
            TypeExprKind::Named(_, arguments) => arguments.iter().collect(),
            TypeExprKind::Function(_, parameters, result) => parameters
                .iter()
                .chain(std::iter::once(result.as_ref()))
                .collect(),
            TypeExprKind::Reference(_, _, inner) => vec![inner.as_ref()],
            TypeExprKind::Primitive(_)
            | TypeExprKind::Hole
            | TypeExprKind::Where(..)
            | TypeExprKind::With(..)
            | TypeExprKind::Trait(_) => Vec::new(),
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn named_bindings(&self) -> Vec<BindingId> {
        let mut named = Vec::new();
        self.collect_named(&mut named);
        named
    }

    fn collect_named(&self, named: &mut Vec<BindingId>) {
        match self.kind() {
            TypeExprKind::Named(binding, _) | TypeExprKind::Trait(binding) => named.push(*binding),
            TypeExprKind::Where(binders, _) => {
                for bound in binders.iter().flat_map(|binder| &binder.bounds) {
                    bound.collect_named(named);
                }
            }
            _ => {}
        }
        for part in self.named_parts() {
            part.collect_named(named);
        }
    }

    fn named_parts(&self) -> Vec<&TypeExpr> {
        match self.kind() {
            TypeExprKind::Where(_, body) => vec![body.as_ref()],
            TypeExprKind::With(implementor, argument) => {
                vec![implementor.as_ref(), argument.as_ref()]
            }
            _ => self.parts(),
        }
    }

    pub fn replacing(&self, variable: BindingId, replacement: &TypeExpr) -> TypeExpr {
        let kind = match self.kind() {
            TypeExprKind::Named(binding, _) if *binding == variable => replacement.kind().clone(),
            TypeExprKind::Named(binding, arguments) => TypeExprKind::Named(
                *binding,
                arguments
                    .iter()
                    .map(|argument| argument.replacing(variable, replacement))
                    .collect(),
            ),
            TypeExprKind::Function(kind, parameters, result) => TypeExprKind::Function(
                *kind,
                parameters
                    .iter()
                    .map(|parameter| parameter.replacing(variable, replacement))
                    .collect(),
                Box::new(result.replacing(variable, replacement)),
            ),
            TypeExprKind::Reference(kind, lifetime, inner) => TypeExprKind::Reference(
                *kind,
                *lifetime,
                Box::new(inner.replacing(variable, replacement)),
            ),
            other => other.clone(),
        };
        TypeExpr::new(kind, self.span)
    }
}

#[cfg(test)]
mod named_binding_tests {
    use super::{
        BindingId, ClosureKind, PrimitiveType, ReferenceKind, TypeBinder, TypeBound, TypeExpr,
        TypeExprKind,
    };
    use crate::source::{SourceId, Span};

    struct NamedFixture;

    impl NamedFixture {
        fn span() -> Span {
            Span::new(SourceId::new(0), 0..0)
        }

        fn type_expr(kind: TypeExprKind) -> TypeExpr {
            TypeExpr::new(kind, Self::span())
        }

        fn named(index: u32) -> TypeExpr {
            Self::type_expr(TypeExprKind::Named(
                BindingId::from_index(index),
                Vec::new(),
            ))
        }

        fn bound(index: u32, argument: Option<TypeExpr>) -> TypeBound {
            TypeBound {
                binding: BindingId::from_index(index),
                argument,
            }
        }

        fn indices(named: Vec<BindingId>) -> Vec<u32> {
            named.into_iter().map(BindingId::index).collect()
        }
    }

    #[test]
    fn a_where_type_names_its_bounds_their_arguments_and_its_body() {
        let binder = TypeBinder {
            variable: BindingId::from_index(9),
            bounds: vec![NamedFixture::bound(1, Some(NamedFixture::named(2)))],
            span: NamedFixture::span(),
            lifetime: false,
        };
        let body = NamedFixture::type_expr(TypeExprKind::Function(
            ClosureKind::Fn,
            vec![NamedFixture::named(3)],
            Box::new(NamedFixture::type_expr(TypeExprKind::Primitive(
                PrimitiveType::Bool,
            ))),
        ));
        let typed = NamedFixture::type_expr(TypeExprKind::Where(vec![binder], Box::new(body)));
        assert_eq!(NamedFixture::indices(typed.named_bindings()), vec![1, 2, 3]);
    }

    #[test]
    fn a_trait_paired_with_a_second_type_names_both() {
        let pair = NamedFixture::type_expr(TypeExprKind::With(
            Box::new(NamedFixture::named(4)),
            Box::new(NamedFixture::type_expr(TypeExprKind::Trait(
                BindingId::from_index(5),
            ))),
        ));
        assert_eq!(NamedFixture::indices(pair.named_bindings()), vec![4, 5]);
    }

    #[test]
    fn a_bound_names_its_trait_before_its_argument() {
        let bound = NamedFixture::bound(6, Some(NamedFixture::named(7)));
        assert_eq!(NamedFixture::indices(bound.named_bindings()), vec![6, 7]);
    }

    #[test]
    fn a_named_type_names_its_arguments() {
        let held = NamedFixture::type_expr(TypeExprKind::Named(
            BindingId::from_index(8),
            vec![NamedFixture::named(9)],
        ));
        assert_eq!(NamedFixture::indices(held.named_bindings()), vec![8, 9]);
    }

    #[test]
    fn a_function_and_a_reference_name_what_they_hold() {
        let made = NamedFixture::type_expr(TypeExprKind::Function(
            ClosureKind::Fn,
            Vec::new(),
            Box::new(NamedFixture::named(10)),
        ));
        let borrowed = NamedFixture::type_expr(TypeExprKind::Reference(
            ReferenceKind::Shared,
            None,
            Box::new(NamedFixture::named(11)),
        ));
        assert_eq!(NamedFixture::indices(made.named_bindings()), vec![10]);
        assert_eq!(NamedFixture::indices(borrowed.named_bindings()), vec![11]);
    }
}

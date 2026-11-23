use crate::core::{BindingId, ReferenceKind, TypeExpr, TypeExprKind};
use crate::types::{GenericId, Type};

pub struct InlineDeclaration {
    variables: Vec<BindingId>,
}

impl InlineDeclaration {
    pub fn of(written: &TypeExpr) -> Option<Type> {
        match written.kind() {
            TypeExprKind::Where(binders, body) => {
                let variables = binders
                    .iter()
                    .filter(|binder| !binder.lifetime)
                    .map(|binder| binder.variable)
                    .collect();
                Self { variables }.lower(body)
            }
            _ => Self {
                variables: Vec::new(),
            }
            .lower(written),
        }
    }

    fn lower(&self, written: &TypeExpr) -> Option<Type> {
        match written.kind() {
            TypeExprKind::Primitive(primitive) => Some(Type::Primitive(*primitive)),
            TypeExprKind::Named(binding, arguments) => self.named(*binding, arguments),
            TypeExprKind::Function(kind, parameters, result) => Some(Type::Function(
                *kind,
                self.all(parameters)?,
                Box::new(self.lower(result)?),
            )),
            TypeExprKind::Reference(ReferenceKind::Shared, _, inner) => {
                Some(Type::shared(self.lower(inner)?))
            }
            TypeExprKind::Reference(ReferenceKind::Mutable, _, inner) => {
                Some(Type::mutable(self.lower(inner)?))
            }
            _ => None,
        }
    }

    fn named(&self, binding: BindingId, arguments: &[TypeExpr]) -> Option<Type> {
        match self
            .variables
            .iter()
            .position(|variable| *variable == binding)
        {
            Some(index) if arguments.is_empty() => Some(Type::Generic(GenericId::new(index))),
            _ => Some(Type::Named(binding, self.all(arguments)?)),
        }
    }

    fn all(&self, written: &[TypeExpr]) -> Option<Vec<Type>> {
        written
            .iter()
            .map(|argument| self.lower(argument))
            .collect()
    }
}

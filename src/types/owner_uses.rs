use super::field_reader::FieldReader;
use super::ty::Type;
use super::typed::{
    Borrow, TypedArgument, TypedDefinition, TypedExpr, TypedExprKind, TypedLeaf, TypedStructExpr,
};
use crate::core::BindingId;

pub(super) struct OwnerUses<'r> {
    reader: &'r FieldReader,
}

impl<'r> OwnerUses<'r> {
    pub(super) fn new(reader: &'r FieldReader) -> Self {
        Self { reader }
    }

    pub(super) fn take_owner_by_value(&self, definition: &TypedDefinition) -> bool {
        let TypedExprKind::Fn(lambda) = &definition.value.kind else {
            return false;
        };
        let owner = Type::Generic(self.reader.owner);
        let mut owners = (lambda.params.iter()).filter(|param| param.ty == owner);
        let body = lambda.body.items.iter().map(|item| item.expr());
        owners.any(|param| {
            body.clone()
                .any(|expr| Self::uses_by_value(expr, param.binding))
        })
    }

    fn uses_by_value(expr: &TypedExpr, owner: BindingId) -> bool {
        match &expr.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => *binding == owner,
            TypedExprKind::Struct(TypedStructExpr::Field(value, _))
            | TypedExprKind::Borrow(_, value)
                if Self::is_owner(value, owner) =>
            {
                false
            }
            TypedExprKind::Call(callee, arguments) => {
                Self::uses_by_value(callee, owner)
                    || (arguments.iter()).any(|argument| Self::passes_by_value(argument, owner))
            }
            TypedExprKind::Assign(assignment) => {
                (assignment.arguments.iter()).any(|argument| Self::passes_by_value(argument, owner))
            }
            _ => (expr.children().into_iter()).any(|child| Self::uses_by_value(child, owner)),
        }
    }

    fn passes_by_value(argument: &TypedArgument, owner: BindingId) -> bool {
        let lent = matches!(argument.borrow, Borrow::Shared | Borrow::Mutable)
            && Self::is_owner(&argument.value, owner);
        !lent && Self::uses_by_value(&argument.value, owner)
    }

    fn is_owner(value: &TypedExpr, owner: BindingId) -> bool {
        matches!(value.kind, TypedExprKind::Leaf(TypedLeaf::Local(local)) if local == owner)
    }
}

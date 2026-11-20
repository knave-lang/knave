use super::field_reader::FieldReader;
use super::ty::Type;
use super::typed::{TypedDefinition, TypedExpr, TypedExprKind, TypedLeaf, TypedStructExpr};
use super::unborrowed::Unborrowed;
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
        let mut owners = (lambda.params.iter()).filter(|param| Unborrowed::of(&param.ty) == &owner);
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
                if Self::is_owner(value, owner) =>
            {
                false
            }
            _ => (expr.children().into_iter()).any(|child| Self::uses_by_value(child, owner)),
        }
    }

    fn is_owner(value: &TypedExpr, owner: BindingId) -> bool {
        matches!(value.kind, TypedExprKind::Leaf(TypedLeaf::Local(local)) if local == owner)
    }
}

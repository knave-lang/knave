use super::field_reader::FieldReader;
use super::passed_call::PassedCall;
use super::ty::Type;
use super::typed::{Borrow, TypedArgument, TypedExpr, TypedExprKind, TypedStructExpr};
use super::unborrowed::Unborrowed;
use crate::core::ReferenceKind;

pub(super) struct FieldChanges<'r> {
    reader: &'r FieldReader,
}

impl<'r> FieldChanges<'r> {
    pub(super) fn new(reader: &'r FieldReader) -> Self {
        Self { reader }
    }

    pub(super) fn within(&self, expr: &TypedExpr) -> bool {
        self.changes_here(expr) || expr.children().into_iter().any(|child| self.within(child))
    }

    fn changes_here(&self, expr: &TypedExpr) -> bool {
        match &expr.kind {
            TypedExprKind::Borrow(ReferenceKind::Mutable, place) => self.reads_owner(place),
            _ if let Some(PassedCall { callee, arguments }) = PassedCall::of(expr) => {
                self.passes_changed_field(callee, arguments)
            }
            _ => false,
        }
    }

    fn passes_changed_field(&self, callee: &TypedExpr, arguments: &[TypedArgument]) -> bool {
        let parameters = match &callee.ty {
            Type::Function(_, parameters, _) => parameters.as_slice(),
            _ => &[],
        };
        let mut passed = arguments.iter().enumerate();
        passed.any(|(position, argument)| {
            let changing = argument.borrow == Borrow::Mutable
                || matches!(parameters.get(position), Some(Type::Mut(..)));
            changing && self.reads_owner(&argument.value)
        })
    }

    fn reads_owner(&self, place: &TypedExpr) -> bool {
        match &place.kind {
            TypedExprKind::Struct(TypedStructExpr::Field(value, _)) => {
                Unborrowed::of(&value.ty) == &Type::Generic(self.reader.owner)
                    || self.reads_owner(value)
            }
            _ => false,
        }
    }
}

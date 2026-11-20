use super::field_reader::FieldReader;
use super::mentioned_generics::MentionedGenerics;
use super::ty::{GenericId, Type};
use super::typed::{TypedExpr, TypedExprKind, TypedStructExpr};
use super::unborrowed::Unborrowed;

pub(super) struct OpenReadHolders {
    open: Vec<GenericId>,
}

impl OpenReadHolders {
    pub(super) fn of(reader: &FieldReader) -> Self {
        Self {
            open: MentionedGenerics::of(&reader.read).into_ids(),
        }
    }

    pub(super) fn are_read_within(&self, expr: &TypedExpr) -> bool {
        self.reads_holder(expr)
            || (expr.children().into_iter()).any(|child| self.are_read_within(child))
    }

    fn reads_holder(&self, expr: &TypedExpr) -> bool {
        let TypedExprKind::Struct(TypedStructExpr::Field(value, _)) = &expr.kind else {
            return false;
        };
        match Unborrowed::of(&value.ty) {
            Type::Named(_, arguments) => (arguments.iter())
                .any(|argument| self.open.iter().any(|generic| argument.mentions(*generic))),
            _ => false,
        }
    }
}

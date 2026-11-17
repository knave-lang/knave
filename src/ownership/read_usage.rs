use crate::types::{CopyRules, Type, TypedExpr};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ReadUsage {
    Move,
    Borrow,
}

impl ReadUsage {
    pub fn moves_field(self, read: &TypedExpr, copy: &CopyRules) -> bool {
        self == ReadUsage::Move && !matches!(read.ty, Type::Ref(..)) && !copy.is_copy(&read.ty)
    }
}

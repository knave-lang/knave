use crate::types::{CopyRules, TypedExpr};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ReadUsage {
    Move,
    Borrow,
}

impl ReadUsage {
    pub fn moves_field(self, read: &TypedExpr, copy: &CopyRules) -> bool {
        self == ReadUsage::Move && !copy.is_copy(&read.ty)
    }
}

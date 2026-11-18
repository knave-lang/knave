use crate::core::FieldSpelling;
use crate::types::{CopyRules, FieldPlace, Type, TypedExpr};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ReadUsage {
    Move,
    Borrow,
}

impl ReadUsage {
    pub fn moves_field(self, read: &TypedExpr, place: &FieldPlace, copy: &CopyRules) -> bool {
        let shared = |ty: &Type| matches!(ty, Type::Ref(..));
        let reads_pair_part = place
            .path
            .last()
            .is_some_and(|label| matches!(label.spelling, FieldSpelling::Pair(_)));
        self == ReadUsage::Move
            && reads_pair_part
            && !shared(place.root_type)
            && !shared(&read.ty)
            && !copy.is_copy(&read.ty)
    }
}

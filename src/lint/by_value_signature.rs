use crate::core::TypeExpr;

pub struct ByValueSignature {
    pub signature: TypeExpr,
    pub positions: Vec<usize>,
}

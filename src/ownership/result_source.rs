use crate::types::{PositionTie, TypedExpr};

pub struct ResultSource<'e> {
    pub tie: PositionTie,
    pub argument: &'e TypedExpr,
}

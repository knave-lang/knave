use super::holdings::Holdings;
use super::lender::Lender;
use super::loan_rules::LoanRules;
use super::loop_heads::LoopHeads;
use super::point_id::PointId;
use super::walked::Walked;
use crate::types::{TypedBinder, TypedBlock, TypedExpr};

pub trait ExpressionWalk<'w> {
    fn expr(&mut self, expr: &'w TypedExpr, at: PointId) -> Walked;

    fn join(&mut self, before: &[PointId]) -> PointId;

    fn lender(&mut self, point: PointId) -> Lender<'_>;

    fn rules(&self) -> &'w LoanRules<'w>;

    fn link(&mut self, before: PointId, after: PointId);

    fn block(&mut self, block: &'w TypedBlock, at: PointId) -> Walked;

    fn define(&mut self, point: PointId, binder: &TypedBinder, held: &Holdings);

    fn loops(&mut self) -> &mut LoopHeads<'w>;
}

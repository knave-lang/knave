use super::lender::Lender;
use super::loan_rules::LoanRules;
use super::point_id::PointId;
use super::walked::Walked;
use crate::types::TypedExpr;

pub trait ExpressionWalk<'w> {
    fn expr(&mut self, expr: &'w TypedExpr, at: PointId) -> Walked;

    fn join(&mut self, before: &[PointId]) -> PointId;

    fn lender(&mut self, point: PointId) -> Lender<'_>;

    fn rules(&self) -> &'w LoanRules<'w>;
}

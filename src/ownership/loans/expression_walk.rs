use super::body_builder::BodyBuilder;
use super::loan_rules::LoanRules;
use super::point_id::PointId;
use super::walked::Walked;
use crate::types::{TypedBlock, TypedExpr, TypedExprKind, TypedStructExpr};

pub trait ExpressionWalk<'w> {
    fn expr(&mut self, expr: &'w TypedExpr, at: PointId) -> Walked;

    fn block(&mut self, block: &'w TypedBlock, at: PointId) -> Walked;

    fn rules(&self) -> &'w LoanRules<'w>;

    fn builder(&mut self) -> &mut BodyBuilder<'w>;

    fn place_root(&mut self, place: &'w TypedExpr, at: PointId) -> Walked {
        match &place.kind {
            TypedExprKind::Struct(TypedStructExpr::Field(owner, _)) => self.place_root(owner, at),
            _ => self.expr(place, at),
        }
    }
}

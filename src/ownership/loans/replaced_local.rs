use super::passed_holdings::PassedHoldings;
use super::point_id::PointId;
use super::value_flow::ValueFlow;
use crate::core::BindingId;
use crate::types::{TypedAssignment, TypedExpr, TypedExprKind, WholeWrite};

#[derive(Clone, Copy)]
pub struct ReplacedLocal {
    local: BindingId,
    value: usize,
}

impl ReplacedLocal {
    pub fn in_expr(expr: &TypedExpr) -> Option<Self> {
        match &expr.kind {
            TypedExprKind::Assign(assignment) => Self::of(assignment),
            _ => None,
        }
    }

    pub fn place(&self) -> usize {
        TypedAssignment::PLACE_POSITION
    }

    pub fn renew(&self, point: PointId, passed: &[PassedHoldings], values: &mut ValueFlow<'_>) {
        if let Some(value) = passed.get(self.value) {
            values.define(point, self.local, &value.all());
        }
    }

    fn of(assignment: &TypedAssignment) -> Option<Self> {
        Some(Self {
            local: WholeWrite::of(assignment)?.replaced_local()?,
            value: assignment.value_position(),
        })
    }
}

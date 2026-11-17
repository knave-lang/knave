use super::moved_place::MovedPlace;
use crate::core::{BindingArena, BindingId};
use crate::source::Span;
use crate::types::{TypedExpr, TypedExprKind, TypedLambda};

pub struct CapturedMove<'a> {
    lambda: Option<&'a TypedLambda>,
    bindings: &'a BindingArena,
}

impl<'a> CapturedMove<'a> {
    pub fn of(closure: &'a TypedExpr, bindings: &'a BindingArena) -> Self {
        let lambda = match &closure.kind {
            TypedExprKind::Fn(lambda) => Some(&**lambda),
            _ => None,
        };
        Self { lambda, bindings }
    }

    pub fn place(&self, capture: Option<BindingId>) -> Option<MovedPlace> {
        let capture = capture?;
        let moved_part = self.lambda.and_then(|lambda| {
            let (_, span) = lambda.facts.moved_capture?;
            let mut body = lambda.body.expressions();
            body.find_map(|expr| self.field_read_at(expr, span))
        });
        Some(moved_part.unwrap_or_else(|| MovedPlace::whole(self.bindings.get(capture).name())))
    }

    fn field_read_at(&self, expr: &TypedExpr, span: Span) -> Option<MovedPlace> {
        match expr.field_place().filter(|place| place.reads_field()) {
            Some(place) if expr.span == span => Some(MovedPlace::of(&place, self.bindings).owned()),
            _ => expr
                .children()
                .into_iter()
                .find_map(|child| self.field_read_at(child, span)),
        }
    }
}

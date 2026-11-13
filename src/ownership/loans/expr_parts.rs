use std::iter;

use crate::types::{TypedExpr, TypedExprKind, TypedStructExpr};

pub struct ExprParts<'e> {
    parts: Vec<&'e TypedExpr>,
}

impl<'e> ExprParts<'e> {
    pub fn of(expr: &'e TypedExpr) -> Self {
        let parts = match &expr.kind {
            TypedExprKind::Try { value, .. }
            | TypedExprKind::Infer(value)
            | TypedExprKind::Deref(value)
            | TypedExprKind::TraitObject(value, _) => vec![value.as_ref()],
            TypedExprKind::Struct(TypedStructExpr::New(_, fields)) => {
                fields.iter().map(|(_, value)| value).collect()
            }
            TypedExprKind::Struct(TypedStructExpr::Update(_, base, fields)) => {
                iter::once(base.as_ref())
                    .chain(fields.iter().map(|(_, value)| value))
                    .collect()
            }
            TypedExprKind::Variant(_, values, _) => values.iter().collect(),
            TypedExprKind::Assoc(map, keys, value) => iter::once(map.as_ref())
                .chain(keys)
                .chain(iter::once(value.as_ref()))
                .collect(),
            _ => Vec::new(),
        };
        Self { parts }
    }

    pub fn into_iter(self) -> impl Iterator<Item = &'e TypedExpr> {
        self.parts.into_iter()
    }
}

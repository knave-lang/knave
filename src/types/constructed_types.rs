use rustc_hash::FxHashMap;

use crate::core::{BindingId, Expr, ExprKind, ItemKind, Lambda, StructExpr};
use crate::expand::Program;

pub(super) struct ConstructedTypes {
    enum_of_variant: FxHashMap<BindingId, BindingId>,
}

impl ConstructedTypes {
    pub(super) fn of(program: &Program) -> Self {
        let enum_of_variant = program
            .modules
            .iter()
            .flat_map(|module| &module.items)
            .filter_map(|item| match &item.kind {
                ItemKind::Enum(binding, _, variants, ..) => Some((*binding, variants)),
                _ => None,
            })
            .flat_map(|(binding, variants)| {
                variants
                    .iter()
                    .map(move |variant| (variant.binding, binding))
            })
            .collect();
        Self { enum_of_variant }
    }

    pub(super) fn in_lambda(&self, lambda: &Lambda) -> Vec<BindingId> {
        let mut constructed = Vec::new();
        let mut pending: Vec<&Expr> = lambda.body.expressions().collect();
        while let Some(expr) = pending.pop() {
            constructed.extend(self.constructed_by(expr));
            pending.extend(expr.children());
        }
        constructed
    }

    fn constructed_by(&self, expr: &Expr) -> Option<BindingId> {
        match expr.kind() {
            ExprKind::Struct(StructExpr::New(binding, _)) => Some(*binding),
            ExprKind::Variant(variant, _) | ExprKind::LabeledVariant(variant, _) => {
                self.enum_of_variant.get(variant).copied()
            }
            _ => None,
        }
    }
}

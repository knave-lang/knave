use super::typed::{TypedBodyItem, TypedControl, TypedExpr, TypedExprKind, TypedLeaf};

pub(super) struct ReturnedLet<'e> {
    block: &'e TypedExpr,
}

impl<'e> ReturnedLet<'e> {
    pub(super) fn new(block: &'e TypedExpr) -> Self {
        Self { block }
    }

    pub(super) fn value(&self) -> Option<&'e TypedExpr> {
        let TypedExprKind::Control(TypedControl::Block(block)) = &self.block.kind else {
            return None;
        };
        let (TypedBodyItem::Expr(tail), items) = block.items.split_last()? else {
            return None;
        };
        let TypedExprKind::Leaf(TypedLeaf::Local(returned)) = &tail.kind else {
            return None;
        };
        if !items.iter().all(Self::binds_a_leaf) {
            return None;
        }
        let (_, value) = items
            .iter()
            .filter_map(TypedBodyItem::as_let)
            .find(|(binder, _)| binder.binding == *returned)?;
        Some(value)
    }

    fn binds_a_leaf(item: &TypedBodyItem) -> bool {
        item.as_let().is_some_and(|(_, value)| {
            matches!(
                value.kind,
                TypedExprKind::Leaf(TypedLeaf::Literal(_) | TypedLeaf::Local(_))
            )
        })
    }
}

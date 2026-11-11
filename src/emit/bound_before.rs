use super::lift_placement::LiftPlacement;
use super::lifted_temporary::LiftedTemporary;
use super::rust_leaf::RustLeaf;
use super::temporary_scope::TemporaryScope;
use super::tree_position::TreePosition;
use crate::types::TypedExpr;

pub struct BoundBefore {
    owner: TreePosition,
}

impl BoundBefore {
    pub fn new(owner: TreePosition) -> Self {
        Self { owner }
    }
}

impl LiftPlacement for BoundBefore {
    fn inner_scope(&self) -> TemporaryScope {
        TemporaryScope::Binding
    }

    fn settles_inside(&self) -> bool {
        true
    }

    fn take(&self, root: &mut TypedExpr, name: String) -> LiftedTemporary {
        let node = self.owner.node_mut(root);
        let named = RustLeaf::new(name.as_str()).typed(node.ty.clone(), node.span);
        LiftedTemporary::bound(name, std::mem::replace(node, named))
    }
}

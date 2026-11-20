use std::collections::BTreeSet;

use crate::core::BindingId;
use crate::types::{TypedExpr, TypedExprKind, TypedLeaf, TypedProgram};

pub struct TraitUseSearch<'p> {
    program: &'p TypedProgram,
}

impl<'p> TraitUseSearch<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub fn collect(&self, expr: &TypedExpr, uses: &mut BTreeSet<BindingId>) {
        if let TypedExprKind::Call(callee, _) = &expr.kind
            && let TypedExprKind::Leaf(TypedLeaf::Global(binding)) = &callee.kind
            && let Some(info) = self.program.declarations.traits().methods().get(binding)
            && info.receiver.is_some()
        {
            uses.insert(info.trait_binding);
        }
        for child in expr.children() {
            self.collect(child, uses);
        }
    }
}

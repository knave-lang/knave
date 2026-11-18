use crate::core::{BindingArena, BindingId, BindingKind, TypeExpr, TypeExprKind};

pub(super) struct WrittenTypeVariables<'a> {
    bindings: &'a BindingArena,
}

impl<'a> WrittenTypeVariables<'a> {
    pub(super) fn new(bindings: &'a BindingArena) -> Self {
        Self { bindings }
    }

    pub(super) fn of(&self, type_expr: &TypeExpr) -> Vec<BindingId> {
        let mut found = Vec::new();
        self.collect(type_expr, &mut found);
        found
    }

    fn collect(&self, type_expr: &TypeExpr, found: &mut Vec<BindingId>) {
        if let TypeExprKind::Named(binding, arguments) = type_expr.kind()
            && arguments.is_empty()
            && matches!(
                self.bindings.get(*binding).kind(),
                BindingKind::TypeVariable
            )
            && !found.contains(binding)
        {
            found.push(*binding);
        }
        for part in type_expr.parts() {
            self.collect(part, found);
        }
    }
}

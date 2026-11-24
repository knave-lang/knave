use rustc_hash::FxHashSet;

use crate::core::{BindingId, ReferenceKind};
use crate::types::{Borrow, TypedExpr, TypedExprKind};

pub struct ChangedLocals {
    changed: FxHashSet<BindingId>,
}

impl ChangedLocals {
    pub fn in_value(value: &TypedExpr) -> Self {
        let mut locals = Self {
            changed: FxHashSet::default(),
        };
        locals.collect(value);
        locals
    }

    pub fn contains(&self, binding: BindingId) -> bool {
        self.changed.contains(&binding)
    }

    fn collect(&mut self, expr: &TypedExpr) {
        match &expr.kind {
            TypedExprKind::Call(_, arguments) => {
                let written = arguments
                    .iter()
                    .filter(|argument| {
                        matches!(argument.borrow, Borrow::Mutable | Borrow::Reborrow)
                    })
                    .filter_map(|argument| argument.value.place_root());
                self.changed.extend(written);
            }
            TypedExprKind::Borrow(ReferenceKind::Mutable, value) => {
                self.changed.extend(value.place_root());
            }
            _ => {}
        }
        for child in expr.children() {
            self.collect(child);
        }
    }
}

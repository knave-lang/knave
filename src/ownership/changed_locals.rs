use rustc_hash::FxHashSet;

use crate::core::BindingId;
use crate::types::{ChangedRoot, PassedCall, TypedExpr};

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
            _ if let Some(PassedCall { arguments, .. }) = PassedCall::of(expr) => self
                .changed
                .extend(arguments.iter().filter_map(ChangedRoot::of_argument)),
            _ => self.changed.extend(ChangedRoot::of_borrow(expr)),
        }
        for child in expr.children() {
            self.collect(child);
        }
    }
}

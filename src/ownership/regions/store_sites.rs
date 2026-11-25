use super::region_rules::RegionRules;
use super::stored_arguments::{Store, StoredArguments};
use crate::types::{TypedArgument, TypedExpr, TypedExprKind, TypedLambda, TypedLeaf};

pub(super) struct StoreSite<'e> {
    pub target: &'e TypedExpr,
    pub value: &'e TypedExpr,
}

pub(super) struct StoreSites<'r, 'e> {
    rules: &'r dyn RegionRules,
    sites: Vec<StoreSite<'e>>,
}

impl<'r, 'e> StoreSites<'r, 'e> {
    pub(super) fn of(rules: &'r dyn RegionRules, lambda: &'e TypedLambda) -> Vec<StoreSite<'e>> {
        let mut found = Self {
            rules,
            sites: Vec::new(),
        };
        for expr in lambda.body.expressions() {
            found.expr(expr);
        }
        found.sites
    }

    fn expr(&mut self, expr: &'e TypedExpr) {
        match &expr.kind {
            TypedExprKind::Call(callee, arguments) => self.call(callee, arguments),
            TypedExprKind::Assign(assignment) => {
                self.call(&assignment.template, &assignment.arguments);
            }
            _ => {}
        }
        for child in expr.children() {
            self.expr(child);
        }
    }

    fn call(&mut self, callee: &TypedExpr, arguments: &'e [TypedArgument]) {
        for store in self.stores_of(callee) {
            self.site(&store, arguments);
        }
    }

    fn stores_of(&self, callee: &TypedExpr) -> Vec<Store> {
        let mut stores = StoredArguments::of(&callee.ty).stores(self.rules.program());
        if let TypedExprKind::Leaf(TypedLeaf::Global(binding)) = callee.kind {
            stores.extend(self.rules.stores_of(binding).positions.iter().cloned());
        }
        stores
    }

    fn site(&mut self, store: &Store, arguments: &'e [TypedArgument]) {
        let (Some(target), Some(value)) = (arguments.get(store.target), arguments.get(store.value))
        else {
            return;
        };
        if self.rules.carries_borrow(&value.value.ty) {
            self.sites.push(StoreSite {
                target: &target.value,
                value: &value.value,
            });
        }
    }
}

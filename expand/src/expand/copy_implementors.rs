use crate::core::{
    CopyTrait, CoreModule, ImplementorRows, ItemKind, RustTraitImplementor, TypeExpr, TypeExprKind,
};
use crate::rust_trait_path::RustTraitPath;

pub(super) struct CopyImplementors {
    row_sets: Vec<ImplementorRows<RustTraitImplementor>>,
}

impl CopyImplementors {
    pub(super) fn in_modules<'m>(modules: impl Iterator<Item = &'m CoreModule>) -> Self {
        let copy = RustTraitPath::from(CopyTrait::PATH);
        let row_sets = modules
            .flat_map(|module| module.items.iter())
            .filter_map(|item| match &item.kind {
                ItemKind::RustTrait(declaration) if declaration.resolved_path == copy => {
                    Some(declaration.implementors.clone())
                }
                _ => None,
            })
            .collect();
        Self { row_sets }
    }

    pub(super) fn lists_primitive(&self, target: &TypeExpr) -> bool {
        let TypeExprKind::Primitive(primitive) = target.kind() else {
            return false;
        };
        self.row_sets
            .iter()
            .any(|rows| rows.lists_primitive(*primitive))
    }
}

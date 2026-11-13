use rustc_hash::FxHashSet;

use crate::core::{BindingId, CoreModule, ItemKind, RustTraitDeclaration};
use crate::source::Span;

#[derive(Clone)]
pub struct ReachedTraits {
    traits: FxHashSet<BindingId>,
}

impl ReachedTraits {
    pub fn new(traits: FxHashSet<BindingId>) -> Self {
        Self { traits }
    }

    pub fn reaches(&self, trait_binding: BindingId) -> bool {
        self.traits.contains(&trait_binding)
    }

    pub fn declarations<'m>(
        &self,
        modules: &'m [CoreModule],
    ) -> impl Iterator<Item = ReachedDeclaration<'m>> {
        modules
            .iter()
            .flat_map(|module| &module.items)
            .filter_map(|item| match &item.kind {
                ItemKind::RustTrait(declaration) => Some(ReachedDeclaration {
                    span: item.span,
                    declaration,
                }),
                _ => None,
            })
            .filter(|reached| self.reaches(reached.declaration.binding))
    }
}

pub struct ReachedDeclaration<'m> {
    pub span: Span,
    pub declaration: &'m RustTraitDeclaration,
}

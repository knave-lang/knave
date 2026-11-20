use std::rc::Rc;

use knave_expand::rust_trait_path::RustTraitPath;
use rustc_hash::FxHashMap;

use super::field_traits::FieldTraits;
use crate::core::{BindingId, RustTraitDeclaration};

#[derive(Default)]
pub struct RustTraitPaths {
    recorded: FxHashMap<BindingId, RecordedPath>,
    pub(super) field_traits: FieldTraits,
}

pub(super) struct RecordedPath {
    pub(super) written: Rc<str>,
    pub(super) resolved: RustTraitPath,
}

impl RustTraitPaths {
    pub fn field_traits(&self) -> &FieldTraits {
        &self.field_traits
    }

    pub fn written(&self, binding: BindingId) -> Option<&Rc<str>> {
        Some(&self.recorded.get(&binding)?.written)
    }

    pub(super) fn recorded(&self, binding: BindingId) -> Option<&RecordedPath> {
        self.recorded.get(&binding)
    }

    pub(super) fn record(&mut self, declaration: &RustTraitDeclaration) {
        let recorded = RecordedPath {
            written: Rc::clone(&declaration.path),
            resolved: declaration.resolved_path.clone(),
        };
        self.recorded.insert(declaration.binding, recorded);
    }
}

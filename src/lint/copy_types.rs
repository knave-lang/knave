use crate::core::BindingId;
use crate::types::{CopyRules, Type, TypedProgram};

pub struct CopyTypes<'t> {
    typed: &'t TypedProgram,
    copy: CopyRules<'t>,
}

impl<'t> CopyTypes<'t> {
    pub fn new(typed: &'t TypedProgram) -> Self {
        Self {
            typed,
            copy: CopyRules::new(&typed.declarations, &typed.generics),
        }
    }

    pub fn possibly_copy(&self, targets: &[BindingId]) -> Vec<BindingId> {
        targets
            .iter()
            .copied()
            .filter(|target| self.is_generic(*target) || self.is_copy(*target))
            .collect()
    }

    fn is_copy(&self, target: BindingId) -> bool {
        self.copy.is_copy(&Type::Named(target, Vec::new()))
    }

    fn is_generic(&self, target: BindingId) -> bool {
        let types = self.typed.declarations.types();
        types
            .structs()
            .get(target)
            .map(|definition| !definition.params.is_empty())
            .or_else(|| {
                types
                    .enums()
                    .get(target)
                    .map(|definition| !definition.params.is_empty())
            })
            .unwrap_or(false)
    }
}

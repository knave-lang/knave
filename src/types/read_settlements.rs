use rustc_hash::FxHashSet;

use super::ty::{Bound, GenericId};
use super::typed::TypedProgram;
use crate::core::CopyTrait;

#[derive(Default)]
pub(super) struct ReadSettlements {
    copied: FxHashSet<GenericId>,
}

impl ReadSettlements {
    pub(super) fn require_copy(&mut self, owner: GenericId) {
        self.copied.insert(owner);
    }

    pub(super) fn apply_to(&self, program: &mut TypedProgram) {
        let copy = (program.declarations.traits().table()).with_paths(&[CopyTrait::PATH]);
        let Some(copy_bound) = copy.first().map(|binding| Bound::plain(*binding)) else {
            return;
        };
        for (id, param) in &mut program.generics {
            if self.copied.contains(id) {
                param.bounds.push(copy_bound.clone());
            }
        }
        let definitions = (program.modules.iter_mut()).flat_map(|module| &mut module.definitions);
        let params = definitions.flat_map(|definition| &mut definition.scheme.params);
        for param in params.filter(|param| self.copied.contains(&param.id)) {
            param.bounds.push(copy_bound.clone());
        }
    }
}

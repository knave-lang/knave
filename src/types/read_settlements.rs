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
        let generics = (program.generics.iter_mut()).filter(|(id, _)| self.copied.contains(id));
        for (_, param) in generics {
            Self::add_once(&mut param.bounds, &copy_bound);
        }
        let definitions = (program.modules.iter_mut()).flat_map(|module| &mut module.definitions);
        let params = definitions.flat_map(|definition| &mut definition.scheme.params);
        for param in params.filter(|param| self.copied.contains(&param.id)) {
            Self::add_once(&mut param.bounds, &copy_bound);
        }
    }

    fn add_once(bounds: &mut Vec<Bound>, copy_bound: &Bound) {
        if !bounds
            .iter()
            .any(|bound| bound.binding == copy_bound.binding)
        {
            bounds.push(copy_bound.clone());
        }
    }
}

use std::rc::Rc;

use rustc_hash::FxHashSet;

use crate::core::ModuleId;
use crate::reader::Symbol;

#[derive(Default)]
pub(super) struct DuplicateDefinitions {
    names: FxHashSet<(ModuleId, Rc<str>)>,
}

impl DuplicateDefinitions {
    pub(super) fn remember(&mut self, module: ModuleId, term: &Symbol) {
        self.names.insert((module, term.shared_name()));
    }

    pub(super) fn holds(&self, module: ModuleId, term: &Symbol) -> bool {
        self.names.contains(&(module, term.shared_name()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DuplicatesFixture;

    impl DuplicatesFixture {
        const MODULE_INDEX: usize = 1;
        const OTHER_MODULE_INDEX: usize = 2;
        const NAME: &str = "helper";

        fn remembered() -> DuplicateDefinitions {
            let mut duplicates = DuplicateDefinitions::default();
            duplicates.remember(ModuleId::new(Self::MODULE_INDEX), &Symbol::new(Self::NAME));
            duplicates
        }
    }

    #[test]
    fn a_remembered_name_is_held_in_its_module_only() {
        let duplicates = DuplicatesFixture::remembered();
        let name = Symbol::new(DuplicatesFixture::NAME);
        assert!(duplicates.holds(ModuleId::new(DuplicatesFixture::MODULE_INDEX), &name));
        assert!(!duplicates.holds(ModuleId::new(DuplicatesFixture::OTHER_MODULE_INDEX), &name));
    }

    #[test]
    fn a_name_never_remembered_is_not_held() {
        let duplicates = DuplicatesFixture::remembered();
        let other = Symbol::new("other");
        assert!(!duplicates.holds(ModuleId::new(DuplicatesFixture::MODULE_INDEX), &other));
    }
}

use rustc_hash::FxHashSet;

use super::visibility::Visibility;
use crate::core::{BindingId, ModuleId};

pub struct ItemVisibilities {
    exported: FxHashSet<BindingId>,
    crate_visible: FxHashSet<BindingId>,
}

impl ItemVisibilities {
    pub fn new(exported: FxHashSet<BindingId>) -> Self {
        Self {
            exported,
            crate_visible: FxHashSet::default(),
        }
    }

    pub fn note_use(&mut self, binding: BindingId, owner: ModuleId, from: ModuleId) {
        if owner != from && !self.exported.contains(&binding) {
            self.crate_visible.insert(binding);
        }
    }

    pub fn of(&self, binding: BindingId) -> Visibility {
        if self.exported.contains(&binding) {
            Visibility::Public
        } else if self.crate_visible.contains(&binding) {
            Visibility::Crate
        } else {
            Visibility::Private
        }
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashSet;

    use super::ItemVisibilities;
    use crate::core::{BindingId, ModuleId};
    use crate::emit::visibility::Visibility;

    struct VisibilityFixture;

    impl VisibilityFixture {
        const EXPORTED: BindingId = BindingId::from_index(0);
        const INTERNAL: BindingId = BindingId::from_index(1);

        fn owner() -> ModuleId {
            ModuleId::new(0)
        }

        fn other_module() -> ModuleId {
            ModuleId::new(1)
        }

        fn visibilities() -> ItemVisibilities {
            ItemVisibilities::new(FxHashSet::from_iter([Self::EXPORTED]))
        }
    }

    #[test]
    fn an_exported_item_is_public() {
        let visibilities = VisibilityFixture::visibilities();
        assert_eq!(
            visibilities.of(VisibilityFixture::EXPORTED),
            Visibility::Public
        );
    }

    #[test]
    fn an_unused_internal_item_is_private() {
        let visibilities = VisibilityFixture::visibilities();
        assert_eq!(
            visibilities.of(VisibilityFixture::INTERNAL),
            Visibility::Private
        );
    }

    #[test]
    fn an_internal_item_used_from_another_module_is_crate_visible() {
        let mut visibilities = VisibilityFixture::visibilities();
        visibilities.note_use(
            VisibilityFixture::INTERNAL,
            VisibilityFixture::owner(),
            VisibilityFixture::other_module(),
        );
        assert_eq!(
            visibilities.of(VisibilityFixture::INTERNAL),
            Visibility::Crate
        );
    }

    #[test]
    fn an_internal_item_used_from_its_own_module_stays_private() {
        let mut visibilities = VisibilityFixture::visibilities();
        visibilities.note_use(
            VisibilityFixture::INTERNAL,
            VisibilityFixture::owner(),
            VisibilityFixture::owner(),
        );
        assert_eq!(
            visibilities.of(VisibilityFixture::INTERNAL),
            Visibility::Private
        );
    }

    #[test]
    fn an_exported_item_used_from_another_module_stays_public() {
        let mut visibilities = VisibilityFixture::visibilities();
        visibilities.note_use(
            VisibilityFixture::EXPORTED,
            VisibilityFixture::owner(),
            VisibilityFixture::other_module(),
        );
        assert_eq!(
            visibilities.of(VisibilityFixture::EXPORTED),
            Visibility::Public
        );
    }
}

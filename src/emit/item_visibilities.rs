use rustc_hash::FxHashSet;

use super::visibility::Visibility;
use crate::core::BindingId;

pub struct ItemVisibilities {
    exported: FxHashSet<BindingId>,
    crate_visible: FxHashSet<BindingId>,
}

impl ItemVisibilities {
    pub fn new(exported: FxHashSet<BindingId>, crate_visible: FxHashSet<BindingId>) -> Self {
        Self {
            exported,
            crate_visible,
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
    use crate::core::BindingId;
    use crate::emit::visibility::Visibility;

    struct VisibilityFixture;

    impl VisibilityFixture {
        const EXPORTED: BindingId = BindingId::from_index(0);
        const INTERNAL: BindingId = BindingId::from_index(1);

        fn used_from_other_modules(used: &[BindingId]) -> ItemVisibilities {
            ItemVisibilities::new(
                FxHashSet::from_iter([Self::EXPORTED]),
                used.iter().copied().collect(),
            )
        }
    }

    #[test]
    fn an_exported_item_is_public() {
        let visibilities = VisibilityFixture::used_from_other_modules(&[]);
        assert_eq!(
            visibilities.of(VisibilityFixture::EXPORTED),
            Visibility::Public
        );
    }

    #[test]
    fn an_internal_item_no_other_module_uses_is_private() {
        let visibilities = VisibilityFixture::used_from_other_modules(&[]);
        assert_eq!(
            visibilities.of(VisibilityFixture::INTERNAL),
            Visibility::Private
        );
    }

    #[test]
    fn an_internal_item_used_from_another_module_is_crate_visible() {
        let visibilities =
            VisibilityFixture::used_from_other_modules(&[VisibilityFixture::INTERNAL]);
        assert_eq!(
            visibilities.of(VisibilityFixture::INTERNAL),
            Visibility::Crate
        );
    }

    #[test]
    fn an_exported_item_used_from_another_module_stays_public() {
        let visibilities =
            VisibilityFixture::used_from_other_modules(&[VisibilityFixture::EXPORTED]);
        assert_eq!(
            visibilities.of(VisibilityFixture::EXPORTED),
            Visibility::Public
        );
    }
}

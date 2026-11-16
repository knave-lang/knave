use super::phase::Phase;
use super::use_mark::UseMark;
use super::{Scope, ScopeSet};
use crate::core::BindingId;

#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct NameEntry {
    scopes: ScopeSet,
    binding: BindingId,
    phase: Phase,
    #[serde(skip)]
    use_mark: UseMark,
}

impl NameEntry {
    pub(super) fn new(scopes: ScopeSet, binding: BindingId, phase: Phase) -> Self {
        Self {
            scopes,
            binding,
            phase,
            use_mark: UseMark::default(),
        }
    }

    pub(super) fn binding(&self) -> BindingId {
        self.binding
    }

    pub(super) fn scopes(&self) -> &ScopeSet {
        &self.scopes
    }

    pub(super) fn is_visible_from_scopes(&self, scopes: &ScopeSet, phase: Phase) -> bool {
        self.phase_matches(phase) && self.scopes.is_subset_of(scopes)
    }

    pub(super) fn is_visible_from_plain_scopes(&self, scopes: &ScopeSet, phase: Phase) -> bool {
        self.phase_matches(phase) && self.scopes.is_plain_subset_of(scopes)
    }

    pub(super) fn use_mark(&self) -> &UseMark {
        &self.use_mark
    }

    pub(super) fn anchor(&self) -> Option<Scope> {
        self.scopes.plain_largest()
    }

    fn phase_matches(&self, phase: Phase) -> bool {
        phase == Phase::CompileTime || self.phase == Phase::Runtime
    }
}

#[cfg(test)]
mod tests {
    use super::NameEntry;
    use crate::core::BindingId;
    use crate::scope::{Phase, ScopeSet};

    #[test]
    fn the_use_mark_fits_in_the_padding_after_the_binding_and_phase() {
        let without_mark = size_of::<ScopeSet>() + size_of::<BindingId>() + size_of::<Phase>();
        assert_eq!(
            size_of::<NameEntry>(),
            without_mark.next_multiple_of(align_of::<ScopeSet>())
        );
    }
}

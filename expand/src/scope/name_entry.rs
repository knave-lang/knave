use super::phase::Phase;
use super::{Scope, ScopeSet};
use crate::core::BindingId;

#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct NameEntry {
    scopes: ScopeSet,
    binding: BindingId,
    phase: Phase,
}

impl NameEntry {
    pub(super) fn new(scopes: ScopeSet, binding: BindingId, phase: Phase) -> Self {
        Self {
            scopes,
            binding,
            phase,
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

    pub(super) fn anchor(&self) -> Option<Scope> {
        self.scopes.plain_largest()
    }

    fn phase_matches(&self, phase: Phase) -> bool {
        phase == Phase::CompileTime || self.phase == Phase::Runtime
    }
}

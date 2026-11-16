use std::ops::Range;

use super::phase::Phase;
use super::{Scope, ScopeSet};
use crate::core::BindingId;

#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct Entry {
    scopes: ScopeSet,
    binding: BindingId,
    phase: Phase,
}

impl Entry {
    pub(super) fn binding(&self) -> BindingId {
        self.binding
    }

    pub(super) fn scope_count(&self) -> usize {
        self.scopes.len()
    }

    fn is_visible_from_scopes(&self, scopes: &ScopeSet, phase: Phase) -> bool {
        self.phase_matches(phase) && self.scopes.is_subset_of(scopes)
    }

    fn is_visible_from_plain_scopes(&self, scopes: &ScopeSet, phase: Phase) -> bool {
        self.phase_matches(phase) && self.scopes.is_plain_subset_of(scopes)
    }

    fn phase_matches(&self, phase: Phase) -> bool {
        phase == Phase::CompileTime || self.phase == Phase::Runtime
    }

    fn anchor(&self) -> Option<Scope> {
        self.scopes.plain_largest()
    }
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(super) struct NameEntries {
    plain_sorted_by_anchor: Vec<Entry>,
    sharing: Vec<Entry>,
}

impl NameEntries {
    const SHORT_LIST_LENGTH: usize = 16;

    pub(super) fn bind(
        &mut self,
        scopes: &ScopeSet,
        binding: BindingId,
        phase: Phase,
    ) -> Result<(), BindingId> {
        if !self.is_plain_search(scopes) {
            return self.bind_sharing(scopes, binding, phase);
        }
        let anchored_end = self.anchored_end(scopes.plain_largest());
        match self.entry_with_scopes(anchored_end, scopes) {
            Some(existing) => Self::existing(existing, binding),
            None => {
                let entry = Entry {
                    scopes: scopes.clone(),
                    binding,
                    phase,
                };
                self.plain_sorted_by_anchor.insert(anchored_end, entry);
                Ok(())
            }
        }
    }

    fn existing(existing: &Entry, binding: BindingId) -> Result<(), BindingId> {
        if existing.binding == binding {
            Ok(())
        } else {
            Err(existing.binding)
        }
    }

    #[inline(never)]
    fn bind_sharing(
        &mut self,
        scopes: &ScopeSet,
        binding: BindingId,
        phase: Phase,
    ) -> Result<(), BindingId> {
        if let Some(existing) = self.all().find(|entry| entry.scopes == *scopes) {
            return Self::existing(existing, binding);
        }
        let entry = Entry {
            scopes: scopes.clone(),
            binding,
            phase,
        };
        if scopes.is_plain() {
            let anchored_end = self.anchored_end(scopes.plain_largest());
            self.plain_sorted_by_anchor.insert(anchored_end, entry);
        } else {
            self.sharing.push(entry);
        }
        Ok(())
    }

    fn anchored_end(&self, anchor: Option<Scope>) -> usize {
        self.plain_sorted_by_anchor
            .partition_point(|entry| entry.anchor() <= anchor)
    }

    fn entry_with_scopes(&self, anchored_end: usize, scopes: &ScopeSet) -> Option<&Entry> {
        let anchor = scopes.plain_largest();
        self.plain_sorted_by_anchor[..anchored_end]
            .iter()
            .rev()
            .take_while(|entry| entry.anchor() == anchor)
            .find(|entry| entry.scopes == *scopes)
    }

    pub(super) fn visible(&self, scopes: &ScopeSet, phase: Phase) -> Vec<&Entry> {
        let mut visible = Vec::new();
        self.visit_visible(scopes, phase, |entry| visible.push(entry));
        visible
    }

    pub(super) fn visit_visible<'entries>(
        &'entries self,
        scopes: &ScopeSet,
        phase: Phase,
        visit: impl FnMut(&'entries Entry),
    ) {
        if self.is_plain_search(scopes) {
            self.visit_plain_visible(scopes, phase, visit);
        } else {
            self.visit_visible_sharing(scopes, phase, visit);
        }
    }

    fn is_plain_search(&self, scopes: &ScopeSet) -> bool {
        self.sharing.is_empty() && scopes.is_plain()
    }

    fn visit_plain_visible<'entries>(
        &'entries self,
        scopes: &ScopeSet,
        phase: Phase,
        mut visit: impl FnMut(&'entries Entry),
    ) {
        let mut visit_if_visible = |entry: &'entries Entry| {
            if entry.is_visible_from_plain_scopes(scopes, phase) {
                visit(entry);
            }
        };
        if self.is_shorter_than_an_anchor_search(scopes) {
            self.plain_sorted_by_anchor
                .iter()
                .for_each(visit_if_visible);
            return;
        }
        for anchor in Self::possible_anchors(scopes) {
            self.plain_sorted_by_anchor[self.anchored(anchor)]
                .iter()
                .for_each(&mut visit_if_visible);
        }
    }

    #[inline(never)]
    fn visit_visible_sharing<'entries>(
        &'entries self,
        scopes: &ScopeSet,
        phase: Phase,
        visit: impl FnMut(&'entries Entry),
    ) {
        self.all()
            .filter(|entry| entry.is_visible_from_scopes(scopes, phase))
            .for_each(visit);
    }

    pub(super) fn unbind(&mut self, binding: BindingId) {
        self.plain_sorted_by_anchor
            .retain(|entry| entry.binding != binding);
        self.sharing.retain(|entry| entry.binding != binding);
    }

    pub(super) fn forget(&mut self, forgotten: &impl Fn(BindingId) -> bool) {
        self.plain_sorted_by_anchor
            .retain(|entry| !forgotten(entry.binding));
        self.sharing.retain(|entry| !forgotten(entry.binding));
    }

    pub(super) fn is_empty(&self) -> bool {
        self.plain_sorted_by_anchor.is_empty() && self.sharing.is_empty()
    }

    pub(super) fn is_within(&self, scopes: &Range<Scope>) -> bool {
        self.all()
            .all(|entry| entry.scopes.iter().any(|scope| scopes.contains(&scope)))
    }

    #[cfg(test)]
    pub(super) fn holds(&self, binding: BindingId) -> bool {
        self.all().any(|entry| entry.binding == binding)
    }

    fn all(&self) -> impl Iterator<Item = &Entry> {
        self.plain_sorted_by_anchor.iter().chain(&self.sharing)
    }

    fn is_shorter_than_an_anchor_search(&self, scopes: &ScopeSet) -> bool {
        self.plain_sorted_by_anchor.len() <= scopes.plain_len().max(Self::SHORT_LIST_LENGTH)
    }

    fn anchored(&self, anchor: Option<Scope>) -> Range<usize> {
        let start = self
            .plain_sorted_by_anchor
            .partition_point(|entry| entry.anchor() < anchor);
        let length =
            self.plain_sorted_by_anchor[start..].partition_point(|entry| entry.anchor() == anchor);
        start..start + length
    }

    fn possible_anchors(scopes: &ScopeSet) -> impl Iterator<Item = Option<Scope>> + '_ {
        std::iter::once(None).chain(scopes.plain_iter().map(Some))
    }
}

#[cfg(test)]
mod tests {
    use super::NameEntries;
    use crate::core::BindingId;
    use crate::scope::{Phase, Scope, ScopeKind, ScopeSet, Scopes};

    struct NameEntriesFixture {
        entries: NameEntries,
        scopes: Vec<Scope>,
        bound: Vec<(Vec<Scope>, BindingId)>,
    }

    impl NameEntriesFixture {
        const SCOPE_COUNT: usize = 5;

        fn bound_under_subsets(count: usize) -> Self {
            let mut supply = Scopes::default();
            let scopes: Vec<Scope> = (0..Self::SCOPE_COUNT)
                .map(|_| supply.fresh(ScopeKind::Local))
                .collect();
            let mut fixture = Self {
                entries: NameEntries::default(),
                scopes,
                bound: Vec::new(),
            };
            (0..count).for_each(|index| fixture.bind_subset(index));
            fixture
        }

        fn bind_subset(&mut self, index: usize) {
            let subset: Vec<Scope> = self
                .scopes
                .iter()
                .enumerate()
                .filter(|(position, _)| index & (1 << position) != 0)
                .map(|(_, scope)| *scope)
                .collect();
            let binding = BindingId::from_index(index as u32);
            self.entries
                .bind(&Self::scope_set(&subset), binding, Phase::Runtime)
                .expect("each subset is bound once");
            self.bound.push((subset, binding));
        }

        fn scope_set(scopes: &[Scope]) -> ScopeSet {
            let mut set = ScopeSet::default();
            scopes.iter().for_each(|scope| set.add(*scope));
            set
        }

        fn reference(&self) -> Vec<Scope> {
            vec![self.scopes[0], self.scopes[1], self.scopes[4]]
        }

        fn visible(&self, reference: &[Scope]) -> Vec<BindingId> {
            let mut visible: Vec<BindingId> = self
                .entries
                .visible(&Self::scope_set(reference), Phase::Runtime)
                .into_iter()
                .map(|entry| entry.binding())
                .collect();
            visible.sort();
            visible
        }

        fn scanned(&self, reference: &[Scope]) -> Vec<BindingId> {
            self.bound
                .iter()
                .filter(|(subset, _)| subset.iter().all(|scope| reference.contains(scope)))
                .map(|(_, binding)| *binding)
                .collect()
        }
    }

    #[test]
    fn a_long_binding_list_finds_what_a_full_scan_finds() {
        let fixture = NameEntriesFixture::bound_under_subsets(NameEntries::SHORT_LIST_LENGTH + 4);
        let reference = fixture.reference();
        assert_eq!(fixture.visible(&reference), fixture.scanned(&reference));
    }
}

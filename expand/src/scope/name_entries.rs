use std::ops::Range;

use super::name_entry::NameEntry;
use super::phase::Phase;
use super::{Scope, ScopeSet};
use crate::core::BindingId;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(super) struct NameEntries {
    plain_sorted_by_anchor: Vec<NameEntry>,
    sharing: Vec<NameEntry>,
}

impl NameEntries {
    const SHORT_LIST_LENGTH: usize = 16;

    #[cfg(test)]
    pub(super) const LONG_LIST_LENGTH: usize = Self::SHORT_LIST_LENGTH + 4;

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
                let entry = NameEntry::new(scopes.clone(), binding, phase);
                self.plain_sorted_by_anchor.insert(anchored_end, entry);
                Ok(())
            }
        }
    }

    fn existing(existing: &NameEntry, binding: BindingId) -> Result<(), BindingId> {
        if existing.binding() == binding {
            Ok(())
        } else {
            Err(existing.binding())
        }
    }

    #[inline(never)]
    fn bind_sharing(
        &mut self,
        scopes: &ScopeSet,
        binding: BindingId,
        phase: Phase,
    ) -> Result<(), BindingId> {
        if let Some(existing) = self.all().find(|entry| entry.scopes() == scopes) {
            return Self::existing(existing, binding);
        }
        let entry = NameEntry::new(scopes.clone(), binding, phase);
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

    fn entry_with_scopes(&self, anchored_end: usize, scopes: &ScopeSet) -> Option<&NameEntry> {
        let anchor = scopes.plain_largest();
        self.plain_sorted_by_anchor[..anchored_end]
            .iter()
            .rev()
            .take_while(|entry| entry.anchor() == anchor)
            .find(|entry| entry.scopes() == scopes)
    }

    pub(super) fn visible(&self, scopes: &ScopeSet, phase: Phase) -> Vec<&NameEntry> {
        let mut visible = Vec::new();
        self.visit_visible(scopes, phase, |entry| visible.push(entry));
        visible
    }

    pub(super) fn visit_visible<'entries>(
        &'entries self,
        scopes: &ScopeSet,
        phase: Phase,
        visit: impl FnMut(&'entries NameEntry),
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
        mut visit: impl FnMut(&'entries NameEntry),
    ) {
        let mut visit_if_visible = |entry: &'entries NameEntry| {
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
        visit: impl FnMut(&'entries NameEntry),
    ) {
        self.all()
            .filter(|entry| entry.is_visible_from_scopes(scopes, phase))
            .for_each(visit);
    }

    pub(super) fn forget(&mut self, forgotten: &impl Fn(BindingId) -> bool) {
        self.plain_sorted_by_anchor
            .retain(|entry| !forgotten(entry.binding()));
        self.sharing.retain(|entry| !forgotten(entry.binding()));
    }

    pub(super) fn is_empty(&self) -> bool {
        self.plain_sorted_by_anchor.is_empty() && self.sharing.is_empty()
    }

    pub(super) fn is_within(&self, scopes: &Range<Scope>) -> bool {
        self.all()
            .all(|entry| entry.scopes().iter().any(|scope| scopes.contains(&scope)))
    }

    #[cfg(test)]
    pub(super) fn holds(&self, binding: BindingId) -> bool {
        self.all().any(|entry| entry.binding() == binding)
    }

    fn all(&self) -> impl Iterator<Item = &NameEntry> {
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
pub(super) mod tests {
    use super::NameEntries;
    use crate::core::BindingId;
    use crate::scope::{Phase, Scope, ScopeKind, ScopeSet, Scopes};

    pub(in crate::scope) struct NameEntriesFixture {
        entries: NameEntries,
        scopes: Vec<Scope>,
        bound: Vec<(Vec<Scope>, BindingId)>,
    }

    impl NameEntriesFixture {
        pub(in crate::scope) const SCOPE_COUNT: usize = 5;

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
            let subset = Self::subset(&self.scopes, index);
            let binding = BindingId::from_index(index as u32);
            self.entries
                .bind(&ScopeSet::of(&subset), binding, Phase::Runtime)
                .expect("each subset is bound once");
            self.bound.push((subset, binding));
        }

        pub(in crate::scope) fn subset(scopes: &[Scope], index: usize) -> Vec<Scope> {
            scopes
                .iter()
                .enumerate()
                .filter(|(position, _)| index & (1 << position) != 0)
                .map(|(_, scope)| *scope)
                .collect()
        }

        pub(in crate::scope) fn last_bound_subset(scopes: &[Scope]) -> Vec<Scope> {
            Self::subset(scopes, NameEntries::LONG_LIST_LENGTH - 1)
        }

        fn reference(&self) -> Vec<Scope> {
            Self::last_bound_subset(&self.scopes)
        }

        fn visible(&self, reference: &[Scope]) -> Vec<BindingId> {
            let mut visible: Vec<BindingId> = self
                .entries
                .visible(&ScopeSet::of(reference), Phase::Runtime)
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
        let fixture = NameEntriesFixture::bound_under_subsets(NameEntries::LONG_LIST_LENGTH);
        let reference = fixture.reference();
        assert_eq!(fixture.visible(&reference), fixture.scanned(&reference));
    }
}

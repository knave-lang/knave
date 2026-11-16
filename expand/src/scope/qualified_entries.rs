use super::name_entry::NameEntry;
use crate::core::BindingId;

pub struct QualifiedEntries<'t> {
    entries: Vec<&'t NameEntry>,
}

impl<'t> QualifiedEntries<'t> {
    pub(super) fn new(entries: Vec<&'t NameEntry>) -> Self {
        Self { entries }
    }

    pub fn bindings(&self) -> Vec<BindingId> {
        let mut bindings: Vec<BindingId> =
            self.entries.iter().map(|entry| entry.binding()).collect();
        bindings.sort();
        bindings.dedup();
        bindings
    }

    pub fn retain(&mut self, keep: impl Fn(BindingId) -> bool) {
        self.entries.retain(|entry| keep(entry.binding()));
    }

    pub fn mark_uses(&self) {
        self.entries
            .iter()
            .for_each(|entry| entry.use_mark().mark());
    }
}

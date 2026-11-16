use super::name_entry::NameEntry;
use super::resolution::Resolution;
use crate::core::BindingId;

#[derive(Default)]
pub struct LargestCandidates<'t> {
    size: usize,
    first: Option<&'t NameEntry>,
    others: Vec<BindingId>,
}

impl<'t> LargestCandidates<'t> {
    #[inline(always)]
    pub(super) fn consider(&mut self, entry: &'t NameEntry) {
        let size = entry.scopes().len();
        match self.first {
            Some(_) if size < self.size => {}
            Some(first) if size == self.size => self.add_equal(first.binding(), entry.binding()),
            _ => self.replace(size, entry),
        }
    }

    #[inline(always)]
    fn add_equal(&mut self, first: BindingId, binding: BindingId) {
        if binding != first {
            self.others.push(binding);
        }
    }

    #[inline(always)]
    fn replace(&mut self, size: usize, entry: &'t NameEntry) {
        self.size = size;
        self.first = Some(entry);
        self.others.clear();
    }

    #[inline(always)]
    pub fn mark_bound_winner(&self) {
        if let Some(first) = self.first.filter(|_| self.others.is_empty()) {
            first.use_mark().mark();
        }
    }

    pub fn bound_where(&self, decides: impl FnOnce(BindingId) -> bool) -> Option<BindingId> {
        self.bound_then(|binding| decides(binding).then_some(binding))
    }

    pub fn bound_then<T>(&self, answers: impl FnOnce(BindingId) -> Option<T>) -> Option<T> {
        let answer = answers(self.bound()?)?;
        self.mark_bound_winner();
        Some(answer)
    }

    #[inline(always)]
    pub fn bound(&self) -> Option<BindingId> {
        self.first
            .filter(|_| self.others.is_empty())
            .map(NameEntry::binding)
    }

    #[inline(always)]
    pub fn is_unbound(&self) -> bool {
        self.first.is_none()
    }

    #[inline(always)]
    pub fn resolution(&self) -> Resolution {
        match (self.first, self.bound()) {
            (None, _) => Resolution::Unbound,
            (Some(_), Some(binding)) => Resolution::Bound(binding),
            (Some(first), None) => Resolution::Ambiguous(self.distinct(first.binding())),
        }
    }

    fn distinct(&self, first: BindingId) -> Vec<BindingId> {
        let mut bindings = self.others.clone();
        bindings.push(first);
        bindings.sort();
        bindings.dedup();
        bindings
    }
}

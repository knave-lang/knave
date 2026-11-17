use rustc_hash::{FxHashMap, FxHashSet};

use super::moved_place::MovedPlace;
use super::moved_values::MoveSite;
use super::pending_diagnostic::PendingDiagnostic;
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

#[derive(Default)]
pub struct BoundFieldMoves {
    bound_names: FxHashMap<Span, BindingId>,
    moved_names: FxHashSet<BindingId>,
}

impl BoundFieldMoves {
    pub fn bind(&mut self, value: Span, name: BindingId) {
        self.bound_names.insert(value, name);
    }

    pub fn record_move(&mut self, name: BindingId) {
        self.moved_names.insert(name);
    }

    pub fn field_site(&self, span: Span, moved: MovedPlace) -> MoveSite {
        let place = if self.bound_names.contains_key(&span) {
            moved
        } else {
            moved.owned()
        };
        MoveSite::Field { span, place }
    }

    pub fn offer(
        &self,
        site: &MoveSite,
        offered: Diagnostic,
        owned: impl FnOnce(&MoveSite) -> Diagnostic,
    ) -> PendingDiagnostic {
        match self.owned_site(site) {
            Some((bound, owned_site)) => {
                let owned = owned(&owned_site);
                PendingDiagnostic::BorrowOffered {
                    offered,
                    owned,
                    bound,
                }
            }
            None => PendingDiagnostic::Final(offered),
        }
    }

    pub fn settle(self, pending: Vec<PendingDiagnostic>) -> Vec<Diagnostic> {
        let moved_names = &self.moved_names;
        pending
            .into_iter()
            .map(|diagnostic| diagnostic.settled(moved_names))
            .collect()
    }

    fn owned_site(&self, site: &MoveSite) -> Option<(BindingId, MoveSite)> {
        let MoveSite::Field { span, place } = site else {
            return None;
        };
        place.borrowed()?;
        let bound = *self.bound_names.get(span)?;
        let place = place.clone().owned();
        Some((bound, MoveSite::Field { span: *span, place }))
    }
}

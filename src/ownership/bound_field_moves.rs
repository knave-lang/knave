use rustc_hash::{FxHashMap, FxHashSet};

use super::moved_place::MovedPlace;
use super::moved_values::MoveSite;
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

#[derive(Default)]
pub struct BoundFieldMoves {
    bound_names: FxHashMap<Span, BindingId>,
    moved_names: FxHashSet<BindingId>,
    borrow_helps: Vec<BorrowHelp>,
}

struct BorrowHelp {
    position: usize,
    bound: BindingId,
    owned: Diagnostic,
}

impl BoundFieldMoves {
    pub fn bind(&mut self, value: Span, name: BindingId) {
        self.bound_names.insert(value, name);
    }

    pub fn record_move(&mut self, name: BindingId) {
        self.moved_names.insert(name);
    }

    pub fn field_site(&self, span: Span, moved: MovedPlace) -> MoveSite {
        let place = match self.bound_names.contains_key(&span) {
            true => moved,
            false => moved.owned(),
        };
        MoveSite::Field { span, place }
    }

    pub fn owned_site(&self, site: &MoveSite) -> Option<(BindingId, MoveSite)> {
        let MoveSite::Field { span, place } = site else {
            return None;
        };
        place.borrowed()?;
        let bound = *self.bound_names.get(span)?;
        let place = place.clone().owned();
        Some((bound, MoveSite::Field { span: *span, place }))
    }

    pub fn defer(&mut self, position: usize, bound: BindingId, owned: Diagnostic) {
        let help = BorrowHelp {
            position,
            bound,
            owned,
        };
        self.borrow_helps.push(help);
    }

    pub fn settle(self, mut diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
        for help in self.borrow_helps {
            if self.moved_names.contains(&help.bound) {
                diagnostics[help.position] = help.owned;
            }
        }
        diagnostics
    }
}

use rustc_hash::FxHashSet;

use super::indexed_points::IndexedPoints;
use super::loan_id::LoanId;
use super::place_access::PlaceAccess;
use super::point_id::PointId;
use crate::core::BindingId;
use crate::source::Span;

pub struct PointFacts<'p> {
    facts: &'p IndexedPoints,
    point: PointId,
}

impl<'p> PointFacts<'p> {
    pub fn new(facts: &'p IndexedPoints, point: PointId) -> Self {
        Self { facts, point }
    }

    pub fn narrow_to_live_before(&self, live: &mut FxHashSet<BindingId>) {
        for defined in self.facts.defined.at(self.point) {
            live.remove(defined);
        }
        live.extend(
            self.facts
                .used
                .at(self.point)
                .iter()
                .map(|used| used.binding),
        );
    }

    pub fn renews(&self, binding: BindingId) -> bool {
        self.facts.defined.at(self.point).contains(&binding)
            || self.facts.replaced.at(self.point).contains(&binding)
    }

    pub fn use_of(&self, binding: BindingId) -> Option<Span> {
        self.facts
            .used
            .at(self.point)
            .iter()
            .find(|used| used.binding == binding)
            .map(|used| used.span)
    }

    pub fn received(&self) -> &'p [LoanId] {
        self.facts.received.at(self.point)
    }

    pub fn accesses(&self) -> &'p [PlaceAccess] {
        self.facts.accesses.at(self.point)
    }

    pub fn successors(&self) -> impl Iterator<Item = PointId> + 'p {
        self.facts.successors.at(self.point).iter().copied()
    }
}

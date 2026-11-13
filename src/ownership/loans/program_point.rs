use rustc_hash::FxHashSet;

use super::loan_id::LoanId;
use super::place_access::PlaceAccess;
use super::point_id::PointId;
use crate::core::BindingId;

#[derive(Default)]
pub struct ProgramPoint {
    defined: Vec<BindingId>,
    used: Vec<BindingId>,
    received: Vec<LoanId>,
    accesses: Vec<PlaceAccess>,
    successors: Vec<PointId>,
}

impl ProgramPoint {
    pub fn define(&mut self, binding: BindingId) {
        self.defined.push(binding);
    }

    pub fn using(&mut self, binding: BindingId) {
        self.used.push(binding);
    }

    pub fn receive(&mut self, loans: impl IntoIterator<Item = LoanId>) {
        self.received.extend(loans);
    }

    pub fn access(&mut self, access: PlaceAccess) {
        self.accesses.push(access);
    }

    pub fn precede(&mut self, successor: PointId) {
        self.successors.push(successor);
    }

    pub fn facts(&self) -> PointFacts<'_> {
        PointFacts { point: self }
    }
}

pub struct PointFacts<'p> {
    point: &'p ProgramPoint,
}

impl<'p> PointFacts<'p> {
    pub fn live_before(&self, mut live: FxHashSet<BindingId>) -> FxHashSet<BindingId> {
        for defined in &self.point.defined {
            live.remove(defined);
        }
        live.extend(self.point.used.iter().copied());
        live
    }

    pub fn received(&self) -> &'p [LoanId] {
        &self.point.received
    }

    pub fn accesses(&self) -> &'p [PlaceAccess] {
        &self.point.accesses
    }

    pub fn successors(&self) -> impl Iterator<Item = PointId> + 'p {
        self.point.successors.iter().copied()
    }
}

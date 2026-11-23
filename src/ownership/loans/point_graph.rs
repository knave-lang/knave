use std::collections::VecDeque;

use rustc_hash::FxHashSet;

use super::indexed_points::IndexedPoints;
use super::point_facts::PointFacts;
use super::point_id::PointId;
use crate::core::BindingId;
use crate::source::Span;

pub struct PointGraph {
    facts: IndexedPoints,
}

impl PointGraph {
    pub fn new(facts: IndexedPoints) -> Self {
        Self { facts }
    }

    pub fn points(&self) -> impl DoubleEndedIterator<Item = (PointId, PointFacts<'_>)> + '_ {
        (0..self.facts.successors.points()).map(|index| {
            let point = PointId::new(index);
            (point, self.facts(point))
        })
    }

    pub fn first_use_after(&self, start: PointId, binding: BindingId) -> Option<Span> {
        let mut reached = FxHashSet::default();
        let mut pending: VecDeque<PointId> = self.facts(start).successors().collect();
        while let Some(point) = pending.pop_front() {
            if !reached.insert(point) {
                continue;
            }
            let facts = self.facts(point);
            if let Some(span) = facts.use_of(binding) {
                return Some(span);
            }
            pending.extend(facts.successors());
        }
        None
    }

    pub fn reachable_from(&self, start: PointId) -> FxHashSet<PointId> {
        let mut reached = FxHashSet::default();
        let mut pending = vec![start];
        while let Some(point) = pending.pop() {
            if reached.insert(point) {
                pending.extend(self.facts(point).successors());
            }
        }
        reached
    }

    fn facts(&self, point: PointId) -> PointFacts<'_> {
        PointFacts::new(&self.facts, point)
    }
}


use rustc_hash::FxHashMap;

use super::bound_origins::{BoundOrigin, BoundOrigins};
use super::inferred_read::InferredRead;
use crate::source::Span;

#[derive(Clone, Debug, Default)]
pub(super) struct InferredAt(Vec<InferredRead>);

impl InferredAt {
    pub(super) fn of(origins: &BoundOrigins) -> Self {
        let reads = origins.as_slice().iter().filter_map(|origin| match origin {
            BoundOrigin::InferredRead(read) => Some(*read),
            _ => None,
        });
        Self(reads.collect())
    }

    pub(super) fn is_inferred(&self) -> bool {
        !self.0.is_empty()
    }

    pub(super) fn first(&self) -> Option<Span> {
        self.0.first().map(InferredRead::read)
    }

    pub(super) fn spans(&self) -> impl Iterator<Item = Span> + '_ {
        self.0.iter().map(InferredRead::read)
    }

    pub(super) fn waits_to_settle(&self) -> bool {
        self.0.iter().any(InferredRead::waits_to_settle)
    }

    pub(super) fn retain_unsettled<V>(&mut self, settled: &FxHashMap<Span, V>) {
        self.0.retain(|read| !settled.contains_key(&read.read()));
    }
}

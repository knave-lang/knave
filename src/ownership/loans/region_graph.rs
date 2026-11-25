use std::collections::hash_map::Entry;

use rustc_hash::{FxHashMap, FxHashSet};

use super::flow_id::FlowId;
use super::point_id::PointId;
use super::reaching_facts::ReachingFacts;
use super::recorded_store::RecordedStore;
use super::stored_loans::StoredLoans;
use crate::core::BindingId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Region(usize);

#[derive(Default)]
pub struct RegionGraph {
    reaching: ReachingFacts,
    entries: FxHashMap<BindingId, Region>,
    flows_into: Vec<FxHashSet<Region>>,
    flowing_at: FxHashMap<PointId, Vec<BindingId>>,
}

impl RegionGraph {
    pub fn linked(reaching: ReachingFacts, stores: &[RecordedStore<StoredLoans>]) -> Self {
        let mut graph = Self {
            flows_into: vec![FxHashSet::default(); reaching.count()],
            entries: FxHashMap::default(),
            flowing_at: FxHashMap::default(),
            reaching,
        };
        graph.enter_sources(stores);
        graph.link_definitions();
        graph.link_stores(stores);
        graph
    }

    pub fn regions_of(&self, binding: BindingId, point: PointId) -> Vec<Region> {
        let reaching: Vec<Region> = self
            .reaching
            .reaching(binding, point)
            .map(Self::of_flow)
            .collect();
        if reaching.is_empty() {
            return self.entries.get(&binding).copied().into_iter().collect();
        }
        reaching
    }

    pub fn reached_from(&self, starts: &[Region]) -> FxHashSet<Region> {
        let mut reached: FxHashSet<Region> = starts.iter().copied().collect();
        let mut pending = starts.to_vec();
        while let Some(region) = pending.pop() {
            for next in &self.flows_into[region.0] {
                if reached.insert(*next) {
                    pending.push(*next);
                }
            }
        }
        reached
    }

    pub fn tracked(&self) -> impl Iterator<Item = BindingId> + '_ {
        self.reaching.bindings().chain(self.entries.keys().copied())
    }

    pub fn flowing_at(&self) -> &FxHashMap<PointId, Vec<BindingId>> {
        &self.flowing_at
    }

    fn enter_sources(&mut self, stores: &[RecordedStore<StoredLoans>]) {
        let defined: Vec<BindingId> = self
            .reaching
            .recorded()
            .flat_map(|(_, recorded)| recorded.sources())
            .collect();
        let stored = stores
            .iter()
            .flat_map(|store| store.held.sources.iter().copied().chain([store.target]));
        for binding in defined.into_iter().chain(stored) {
            self.enter(binding);
        }
    }

    fn enter(&mut self, binding: BindingId) {
        let next = Region(self.flows_into.len());
        if let Entry::Vacant(vacant) = self.entries.entry(binding) {
            vacant.insert(next);
            self.flows_into.push(FxHashSet::default());
        }
    }

    fn link_definitions(&mut self) {
        let mut links = Vec::new();
        for (flow, recorded) in self.reaching.recorded() {
            for source in recorded.sources() {
                self.flowing_at
                    .entry(recorded.point)
                    .or_default()
                    .push(source);
                for from in self.regions_of(source, recorded.point) {
                    links.push((from, Self::of_flow(flow)));
                }
            }
        }
        self.link_all(links);
    }

    fn link_stores(&mut self, stores: &[RecordedStore<StoredLoans>]) {
        let mut links = Vec::new();
        for store in stores {
            let targets = self.regions_of(store.target, store.point);
            for source in &store.held.sources {
                self.flowing_at
                    .entry(store.point)
                    .or_default()
                    .push(*source);
                for from in self.regions_of(*source, store.point) {
                    links.extend(targets.iter().map(|into| (from, *into)));
                }
            }
        }
        self.link_all(links);
    }

    fn link_all(&mut self, links: Vec<(Region, Region)>) {
        for (from, into) in links {
            if from != into {
                self.flows_into[from.0].insert(into);
            }
        }
    }

    fn of_flow(flow: FlowId) -> Region {
        Region(flow.index())
    }
}

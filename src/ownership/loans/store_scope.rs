use rustc_hash::FxHashMap;

use super::fact_row::FactRow;
use super::forward_facts::FactTransfer;
use super::point_graph::PointGraph;
use super::point_id::PointId;
use super::store_holders::StoreHolders;

pub struct StoreScope {
    kept: Vec<FactRow>,
    generated: FxHashMap<PointId, Vec<usize>>,
}

impl StoreScope {
    pub fn of(graph: &PointGraph, holders: &StoreHolders<'_>, store_points: &[PointId]) -> Self {
        let mut generated: FxHashMap<PointId, Vec<usize>> = FxHashMap::default();
        for (store, point) in store_points.iter().enumerate() {
            generated.entry(*point).or_default().push(store);
        }
        Self {
            kept: graph
                .points()
                .map(|(point, _)| holders.kept_at(point))
                .collect(),
            generated,
        }
    }
}

impl FactTransfer for StoreScope {
    fn transfer(&self, point: PointId, facts: &mut FactRow) {
        facts.meet(&self.kept[point.index()]);
        for store in self.generated.get(&point).into_iter().flatten() {
            facts.set(*store);
        }
    }
}

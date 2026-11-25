use super::held_loans::HeldLoans;
use super::holdings::Holdings;
use super::point_graph::PointGraph;
use super::point_id::PointId;
use super::recorded_store::RecordedStore;
use super::store_reach::StoreReach;
use super::value_regions::ValueRegions;
use crate::core::BindingId;

#[derive(Default)]
pub struct BodyRegions {
    values: ValueRegions,
    stores: Vec<RecordedStore>,
    renewals: Vec<(PointId, BindingId)>,
}

impl BodyRegions {
    pub fn flow_into(&mut self, binding: BindingId, held: &Holdings) {
        self.values.flow_into(binding, held);
    }

    pub fn store(&mut self, store: RecordedStore) {
        self.stores.push(store);
    }

    pub fn renew(&mut self, point: PointId, binding: BindingId) {
        self.renewals.push((point, binding));
    }

    pub fn held_loans(self, graph: &PointGraph) -> HeldLoans {
        if self.stores.is_empty() {
            return HeldLoans::new(self.values.resolved());
        }
        let unstored = self.values.clone().resolved();
        let reach = StoreReach::of(graph, &self.stores, &self.renewals);
        let mut values = self.values;
        for store in &self.stores {
            values.flow_into(store.target, &store.held);
        }
        HeldLoans::new(values.resolved()).with_stores(unstored, self.stores, reach)
    }
}

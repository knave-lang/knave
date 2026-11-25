use super::held_loans::HeldLoans;
use super::holdings::Holdings;
use super::point_graph::PointGraph;
use super::recorded_store::RecordedStore;
use super::value_regions::ValueRegions;
use crate::core::BindingId;

#[derive(Default)]
pub struct BodyRegions {
    values: ValueRegions,
    stores: Vec<RecordedStore>,
}

impl BodyRegions {
    pub fn flow_into(&mut self, binding: BindingId, held: &Holdings) {
        self.values.flow_into(binding, held);
    }

    pub fn store(&mut self, store: RecordedStore) {
        self.stores.push(store);
    }

    pub fn held_loans(self, graph: &PointGraph) -> HeldLoans {
        HeldLoans::of_body(self.values, self.stores, graph)
    }
}

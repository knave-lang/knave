use super::held_loans::HeldLoans;
use super::holdings::Holdings;
use super::point_graph::PointGraph;
use super::point_id::PointId;
use super::recorded_flow::RecordedFlow;
use super::recorded_store::RecordedStore;
use super::region_flow::RegionFlow;
use super::value_regions::ValueRegions;
use crate::core::BindingId;

#[derive(Default)]
pub struct BodyRegions {
    values: ValueRegions,
    flows: Vec<RecordedFlow>,
    stores: Vec<RecordedStore>,
}

impl BodyRegions {
    pub fn flow_into(&mut self, point: PointId, binding: BindingId, held: &Holdings) {
        self.values.flow_into(binding, held);
        self.flows.push(RecordedFlow {
            point,
            binding,
            sources: held.values().map(|value| value.binding()).collect(),
        });
    }

    pub fn store(&mut self, store: RecordedStore) {
        self.stores.push(store);
    }

    pub fn held_loans(self, graph: &PointGraph) -> HeldLoans {
        let regions = self.values.resolved();
        let stores = self
            .stores
            .into_iter()
            .map(|store| RecordedStore {
                point: store.point,
                target: store.target,
                held: regions.of_holdings(&store.held),
            })
            .collect();
        HeldLoans::new(regions, RegionFlow::of(graph, self.flows, stores))
    }
}

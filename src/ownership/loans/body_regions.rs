use super::held_loans::HeldLoans;
use super::point_graph::PointGraph;
use super::recorded_flow::RecordedFlow;
use super::recorded_store::RecordedStore;
use super::region_flow::RegionFlow;
use super::value_regions::ValueRegions;

#[derive(Default)]
pub struct BodyRegions {
    flows: Vec<RecordedFlow>,
    stores: Vec<RecordedStore>,
}

impl BodyRegions {
    pub fn flow_into(&mut self, flow: RecordedFlow) {
        self.flows.push(flow);
    }

    pub fn store(&mut self, store: RecordedStore) {
        self.stores.push(store);
    }

    pub fn held_loans(self, graph: &PointGraph) -> HeldLoans {
        let regions = ValueRegions::of_flows(&self.flows);
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

use std::collections::BTreeSet;

use super::holdings::Holdings;
use super::loan_id::LoanId;
use super::point_graph::PointGraph;
use super::point_id::PointId;
use super::recorded_store::RecordedStore;
use super::store_reach::StoreReach;
use super::value_regions::ValueRegions;
use crate::core::BindingId;

pub struct HeldLoans {
    regions: ValueRegions,
    unstored: ValueRegions,
    stores: StoreReach<BTreeSet<LoanId>>,
}

impl HeldLoans {
    pub fn of_body(values: ValueRegions, stores: Vec<RecordedStore>, graph: &PointGraph) -> Self {
        let unstored = if stores.is_empty() {
            ValueRegions::default()
        } else {
            values.clone().resolved()
        };
        let regions = Self::holding_stores(values, &stores).resolved();
        let resolved = stores
            .into_iter()
            .map(|store| RecordedStore {
                point: store.point,
                target: store.target,
                held: regions.of_holdings(&store.held),
            })
            .collect();
        Self {
            regions,
            unstored,
            stores: StoreReach::of(graph, resolved),
        }
    }

    pub fn of(&self, value: BindingId, point: PointId) -> BTreeSet<LoanId> {
        if !self.stores.stores_into(value) {
            return self.regions.of(value);
        }
        let mut loans = self.unstored.of(value);
        for stored in self.stores.held_at(point, value) {
            loans.extend(stored);
        }
        loans
    }

    pub fn of_holdings(&self, held: &Holdings) -> BTreeSet<LoanId> {
        self.regions.of_holdings(held)
    }

    fn holding_stores(mut values: ValueRegions, stores: &[RecordedStore]) -> ValueRegions {
        for store in stores {
            values.flow_into(store.target, &store.held);
        }
        values
    }
}

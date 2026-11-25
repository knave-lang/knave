use std::collections::BTreeSet;

use super::holdings::Holdings;
use super::loan_id::LoanId;
use super::point_id::PointId;
use super::recorded_store::RecordedStore;
use super::store_reach::StoreReach;
use super::value_regions::ValueRegions;
use crate::core::BindingId;

pub struct HeldLoans {
    regions: ValueRegions,
    unstored: ValueRegions,
    stores: Vec<RecordedStore>,
    reach: StoreReach,
}

impl HeldLoans {
    pub fn new(regions: ValueRegions) -> Self {
        Self {
            regions,
            unstored: ValueRegions::default(),
            stores: Vec::new(),
            reach: StoreReach::default(),
        }
    }

    pub fn with_stores(
        self,
        unstored: ValueRegions,
        stores: Vec<RecordedStore>,
        reach: StoreReach,
    ) -> Self {
        Self {
            unstored,
            stores,
            reach,
            ..self
        }
    }

    pub fn of(&self, value: BindingId, point: PointId) -> BTreeSet<LoanId> {
        if self.stores.iter().any(|store| store.target == value) {
            self.held_by_target(value, point)
        } else {
            self.regions.of(value)
        }
    }

    pub fn of_holdings(&self, held: &Holdings) -> BTreeSet<LoanId> {
        self.regions.of_holdings(held)
    }

    fn held_by_target(&self, target: BindingId, point: PointId) -> BTreeSet<LoanId> {
        let reached = self
            .stores
            .iter()
            .enumerate()
            .filter(|(index, store)| store.target == target && self.reach.reaches(point, *index))
            .flat_map(|(_, store)| self.regions.of_holdings(&store.held));
        self.unstored
            .of(target)
            .into_iter()
            .chain(reached)
            .collect()
    }
}

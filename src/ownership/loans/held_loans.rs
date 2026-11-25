use std::collections::BTreeSet;

use super::holdings::Holdings;
use super::loan_id::LoanId;
use super::point_id::PointId;
use super::region_flow::RegionFlow;
use super::value_regions::ValueRegions;
use crate::core::BindingId;

pub struct HeldLoans {
    regions: ValueRegions,
    stored: RegionFlow,
}

impl HeldLoans {
    pub fn new(regions: ValueRegions, stored: RegionFlow) -> Self {
        Self { regions, stored }
    }

    pub fn of(&self, value: BindingId, point: PointId) -> BTreeSet<LoanId> {
        let mut loans = self.regions.of(value);
        for stored in self.stored.held_at(point, value) {
            loans.extend(stored);
        }
        loans
    }

    pub fn of_holdings(&self, held: &Holdings) -> BTreeSet<LoanId> {
        self.regions.of_holdings(held)
    }
}

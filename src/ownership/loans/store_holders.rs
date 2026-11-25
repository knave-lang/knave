use rustc_hash::{FxHashMap, FxHashSet};

use super::fact_row::FactRow;
use super::live_values::LiveValues;
use super::point_id::PointId;
use super::region_graph::{Region, RegionGraph};
use crate::core::BindingId;

pub struct StoreHolders<'h> {
    regions: &'h RegionGraph,
    live: &'h LiveValues,
    stores_by_region: FxHashMap<Region, FactRow>,
    stores: usize,
}

impl<'h> StoreHolders<'h> {
    pub fn new<'s>(
        regions: &'h RegionGraph,
        live: &'h LiveValues,
        reaches: impl ExactSizeIterator<Item = &'s FxHashSet<Region>>,
    ) -> Self {
        let stores = reaches.len();
        let mut stores_by_region: FxHashMap<Region, FactRow> = FxHashMap::default();
        for (store, reach) in reaches.enumerate() {
            for region in reach {
                stores_by_region
                    .entry(*region)
                    .or_insert_with(|| FactRow::sized(stores))
                    .set(store);
            }
        }
        Self {
            regions,
            live,
            stores_by_region,
            stores,
        }
    }

    pub fn kept_at(&self, point: PointId) -> FactRow {
        let live = self.live.before(point);
        let mut kept = FactRow::sized(self.stores);
        let holders = self
            .regions
            .tracked()
            .filter(|binding| live.contains(binding));
        for holder in holders {
            self.keep_held_by(&mut kept, holder, point);
        }
        kept
    }

    fn keep_held_by(&self, kept: &mut FactRow, holder: BindingId, point: PointId) {
        for region in self.regions.regions_of(holder, point) {
            if let Some(stores) = self.stores_by_region.get(&region) {
                kept.absorb(stores);
            }
        }
    }
}

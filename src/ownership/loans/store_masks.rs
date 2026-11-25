use rustc_hash::{FxHashMap, FxHashSet};

use super::fact_row::FactRow;
use super::region_graph::Region;

#[derive(Default)]
pub struct StoreMasks {
    by_region: FxHashMap<Region, FactRow>,
    stores: usize,
}

impl StoreMasks {
    pub fn of<'s>(reaches: impl ExactSizeIterator<Item = &'s FxHashSet<Region>>) -> Self {
        let stores = reaches.len();
        let mut by_region: FxHashMap<Region, FactRow> = FxHashMap::default();
        for (store, reach) in reaches.enumerate() {
            for region in reach {
                by_region
                    .entry(*region)
                    .or_insert_with(|| FactRow::sized(stores))
                    .set(store);
            }
        }
        Self { by_region, stores }
    }

    pub fn held_by(&self, regions: &[Region]) -> FactRow {
        let mut held = FactRow::sized(self.stores);
        for region in regions {
            if let Some(stores) = self.by_region.get(region) {
                held.absorb(stores);
            }
        }
        held
    }
}

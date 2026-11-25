use super::fact_row::FactRow;
use super::live_values::LiveValues;
use super::point_id::PointId;
use super::region_graph::RegionGraph;
use super::store_masks::StoreMasks;

pub struct StoreHolders<'h> {
    regions: &'h RegionGraph,
    live: &'h LiveValues,
    masks: &'h StoreMasks,
}

impl<'h> StoreHolders<'h> {
    pub fn new(regions: &'h RegionGraph, live: &'h LiveValues, masks: &'h StoreMasks) -> Self {
        Self {
            regions,
            live,
            masks,
        }
    }

    pub fn kept_at(&self, point: PointId) -> FactRow {
        let live = self.live.before(point);
        let mut kept = self.masks.held_by(&[]);
        let holders = self
            .regions
            .tracked()
            .filter(|binding| live.contains(binding));
        for holder in holders {
            kept.absorb(&self.masks.held_by(&self.regions.regions_of(holder, point)));
        }
        kept
    }
}

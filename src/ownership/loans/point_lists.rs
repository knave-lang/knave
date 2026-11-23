use super::point_id::PointId;
use super::point_index::PointIndex;

pub struct PointLists<T> {
    entries: Vec<(PointId, T)>,
}

impl<T> Default for PointLists<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<T> PointLists<T> {
    pub fn push(&mut self, point: PointId, item: T) {
        self.entries.push((point, item));
    }

    pub fn indexed(self, points: usize) -> PointIndex<T> {
        PointIndex::of(self.entries, points)
    }
}

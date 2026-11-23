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

    pub fn indexed(mut self, points: usize) -> PointIndex<T> {
        self.entries.sort_by_key(|(point, _)| *point);
        let mut starts = vec![0; points + 1];
        for (point, _) in &self.entries {
            starts[point.index() + 1] += 1;
        }
        for point in 0..points {
            starts[point + 1] += starts[point];
        }
        let items = self.entries.into_iter().map(|(_, item)| item).collect();
        PointIndex::new(starts, items)
    }
}

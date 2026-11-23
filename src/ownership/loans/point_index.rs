use super::point_id::PointId;

pub struct PointIndex<T> {
    starts: Vec<usize>,
    items: Vec<T>,
}

impl<T> PointIndex<T> {
    pub fn of(mut entries: Vec<(PointId, T)>, points: usize) -> Self {
        entries.sort_by_key(|(point, _)| *point);
        let mut starts = vec![0; points + 1];
        for (point, _) in &entries {
            starts[point.index() + 1] += 1;
        }
        for point in 0..points {
            starts[point + 1] += starts[point];
        }
        Self {
            starts,
            items: entries.into_iter().map(|(_, item)| item).collect(),
        }
    }

    pub fn at(&self, point: PointId) -> &[T] {
        &self.items[self.starts[point.index()]..self.starts[point.index() + 1]]
    }

    pub fn points(&self) -> usize {
        self.starts.len() - 1
    }
}

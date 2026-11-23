use super::point_id::PointId;

pub struct PointIndex<T> {
    starts: Vec<usize>,
    items: Vec<T>,
}

impl<T> PointIndex<T> {
    pub fn new(starts: Vec<usize>, items: Vec<T>) -> Self {
        Self { starts, items }
    }

    pub fn at(&self, point: PointId) -> &[T] {
        &self.items[self.starts[point.index()]..self.starts[point.index() + 1]]
    }
}

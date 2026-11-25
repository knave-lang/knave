use super::holdings::Holdings;
use super::point_id::PointId;
use crate::core::BindingId;

pub struct RecordedFlow {
    pub point: PointId,
    pub binding: BindingId,
    pub held: Holdings,
}

impl RecordedFlow {
    pub fn sources(&self) -> impl Iterator<Item = BindingId> + '_ {
        self.held.values().map(|value| value.binding())
    }

    #[cfg(test)]
    pub fn from_sources(point: PointId, binding: BindingId, sources: &[BindingId]) -> Self {
        let held = sources
            .iter()
            .map(|source| Holdings::of_value(*source))
            .fold(Holdings::default(), Holdings::merged);
        Self {
            point,
            binding,
            held,
        }
    }
}

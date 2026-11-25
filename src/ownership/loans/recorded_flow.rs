use super::point_id::PointId;
use crate::core::BindingId;

pub struct RecordedFlow {
    pub point: PointId,
    pub binding: BindingId,
    pub sources: Vec<BindingId>,
}

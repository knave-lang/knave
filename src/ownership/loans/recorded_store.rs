use super::holdings::Holdings;
use super::point_id::PointId;
use crate::core::BindingId;

pub struct RecordedStore {
    pub point: PointId,
    pub target: BindingId,
    pub held: Holdings,
}

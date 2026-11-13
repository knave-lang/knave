use super::holdings::Holdings;
use super::point_id::PointId;

pub struct Walked {
    pub end: PointId,
    pub held: Holdings,
}

impl Walked {
    pub fn at(end: PointId) -> Self {
        Self {
            end,
            held: Holdings::default(),
        }
    }

    pub fn holding(end: PointId, held: Holdings) -> Self {
        Self { end, held }
    }
}

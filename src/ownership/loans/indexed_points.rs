use super::loan_id::LoanId;
use super::place_access::PlaceAccess;
use super::point_id::PointId;
use super::point_index::PointIndex;
use super::value_use::ValueUse;
use crate::core::BindingId;

pub struct IndexedPoints {
    pub defined: PointIndex<BindingId>,
    pub used: PointIndex<ValueUse>,
    pub received: PointIndex<LoanId>,
    pub accesses: PointIndex<PlaceAccess>,
    pub successors: PointIndex<PointId>,
}

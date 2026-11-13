use super::point_id::PointId;
use crate::types::TypedBinder;

pub trait LoopNest<'w> {
    fn enter_loop(&mut self, head: PointId, parameters: &'w [TypedBinder]);

    fn leave_loop(&mut self);

    fn innermost_loop_parameters(&self) -> Option<&'w [TypedBinder]>;

    fn return_to_innermost_loop(&mut self, from: PointId);
}

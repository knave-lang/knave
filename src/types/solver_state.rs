use super::implied_bounds::ImpliedBounds;
use super::unifier::Unifier;

pub(super) struct SolverState<'a> {
    pub(super) unifier: &'a mut Unifier,
    pub(super) implied_bounds: &'a mut ImpliedBounds,
}

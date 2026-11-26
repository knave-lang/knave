use super::unheld_binder::UnheldBinder;

pub(super) struct UnheldAt {
    pub(super) holder_position: usize,
    pub(super) binder_position: usize,
    pub(super) holder_held: bool,
    pub(super) binder: UnheldBinder,
}

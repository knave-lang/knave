#[derive(Clone, Copy)]
pub(super) enum ObligationPass {
    Deferring,
    Final,
}

#[derive(Clone, Copy)]
pub(super) struct ProblemMark {
    position: usize,
}

impl ProblemMark {
    pub(super) fn at(position: usize) -> Self {
        Self { position }
    }

    pub(super) fn position(self) -> usize {
        self.position
    }
}

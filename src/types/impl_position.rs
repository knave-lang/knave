#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct ImplPosition(usize);

impl ImplPosition {
    pub(super) fn all(count: usize) -> impl Iterator<Item = Self> {
        (0..count).map(Self)
    }

    pub(super) fn index(self) -> usize {
        self.0
    }
}

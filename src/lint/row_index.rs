#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowIndex(usize);

impl RowIndex {
    pub fn new(position: usize) -> Self {
        Self(position)
    }

    pub fn position(self) -> usize {
        self.0
    }
}

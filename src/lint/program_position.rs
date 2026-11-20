#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgramPosition(usize);

impl ProgramPosition {
    pub fn nth(index: usize) -> Self {
        Self(index)
    }

    pub fn index(self) -> usize {
        self.0
    }
}

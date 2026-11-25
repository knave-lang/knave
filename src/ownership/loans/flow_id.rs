#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FlowId(usize);

impl FlowId {
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    pub fn index(self) -> usize {
        self.0
    }
}

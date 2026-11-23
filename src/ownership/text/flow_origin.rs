#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowOrigin {
    Written,
    Generated,
}

impl FlowOrigin {
    pub fn is_generated(self) -> bool {
        self == Self::Generated
    }
}

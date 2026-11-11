#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PayloadPosition {
    pub variant: usize,
    pub payload: usize,
}

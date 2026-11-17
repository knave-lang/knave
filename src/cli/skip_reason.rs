#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipReason {
    HiddenFolder,
    BuildFolder,
}

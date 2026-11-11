#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TemporaryScope {
    Statement,
    Binding,
}

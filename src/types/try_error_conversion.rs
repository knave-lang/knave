#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TryErrorConversion {
    Identity,
    Converted,
}

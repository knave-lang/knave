#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PositionTie {
    Lent,
    Held,
    HeldBeneath,
}

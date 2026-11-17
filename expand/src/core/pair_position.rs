#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PairPosition {
    First,
    Second,
}

impl PairPosition {
    pub fn index(self) -> usize {
        match self {
            PairPosition::First => 0,
            PairPosition::Second => 1,
        }
    }

    pub fn part_of<T>(self, parts: &[T]) -> Option<&T> {
        parts.get(self.index())
    }
}

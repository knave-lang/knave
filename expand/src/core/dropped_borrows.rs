#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DroppedBorrows {
    positions: Vec<usize>,
}

impl DroppedBorrows {
    pub fn new(positions: Vec<usize>) -> Self {
        Self { positions }
    }

    pub fn arguments(&self) -> impl Iterator<Item = usize> + '_ {
        self.positions.iter().copied()
    }
}

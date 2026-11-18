#[derive(Clone, Default)]
pub struct ArgumentPositions {
    positions: Vec<usize>,
}

impl ArgumentPositions {
    pub fn new(positions: Vec<usize>) -> Self {
        Self { positions }
    }

    pub fn holds(&self, argument: usize) -> bool {
        self.positions.contains(&argument)
    }
}

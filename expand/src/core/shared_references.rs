#[derive(Default)]
pub struct SharedReferences {
    positions: Vec<usize>,
}

impl SharedReferences {
    pub fn new(positions: Vec<usize>) -> Self {
        Self { positions }
    }

    pub fn holds(&self, argument: usize) -> bool {
        self.positions.contains(&argument)
    }
}

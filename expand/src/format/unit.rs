#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Single(usize),
    Pair(usize, usize),
}

impl Unit {
    pub fn first_entry(self) -> usize {
        match self {
            Unit::Single(entry) | Unit::Pair(entry, _) => entry,
        }
    }

    pub fn last_entry(self) -> usize {
        match self {
            Unit::Single(entry) | Unit::Pair(_, entry) => entry,
        }
    }

    pub fn entries(self) -> impl Iterator<Item = usize> {
        self.first_entry()..=self.last_entry()
    }

    pub fn entry_count(self) -> usize {
        self.last_entry() - self.first_entry() + 1
    }
}

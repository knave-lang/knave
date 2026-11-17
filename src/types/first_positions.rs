#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FirstPositions<const PAST_THE_FIRST: bool>(u64);

pub type FirstPositionsOnly = FirstPositions<false>;

pub type FirstPositionsAndBeyond = FirstPositions<true>;

impl<const PAST_THE_FIRST: bool> FirstPositions<PAST_THE_FIRST> {
    const COUNT: usize = u64::BITS as usize;

    pub fn of(positions: impl IntoIterator<Item = usize>) -> Self {
        positions
            .into_iter()
            .filter(|position| Self::reaches(*position))
            .fold(Self::default(), |first, position| {
                Self(first.0 | 1 << position)
            })
    }

    pub fn reaches(position: usize) -> bool {
        position < Self::COUNT
    }

    pub fn contains(self, position: usize) -> bool {
        match Self::reaches(position) {
            true => self.0 & (1 << position) != 0,
            false => PAST_THE_FIRST,
        }
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    pub fn positions(self) -> impl Iterator<Item = usize> {
        (0..Self::COUNT).filter(move |position| self.0 & (1 << position) != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::{FirstPositionsAndBeyond, FirstPositionsOnly};

    #[test]
    fn a_position_past_the_first_word_takes_the_answer_its_type_names() {
        let only = FirstPositionsOnly::of([3, 64, 70]);
        let beyond = FirstPositionsAndBeyond::of([3]);
        assert_eq!(only.positions().collect::<Vec<_>>(), [3]);
        assert!(!only.contains(64) && !only.contains(4));
        assert!(beyond.contains(64) && !beyond.contains(4));
    }

    #[test]
    fn a_union_keeps_the_positions_of_both() {
        let left = FirstPositionsOnly::of([1, 2]);
        let right = FirstPositionsOnly::of([2, 5]);
        assert_eq!(left.union(right), FirstPositionsOnly::of([1, 2, 5]));
    }
}

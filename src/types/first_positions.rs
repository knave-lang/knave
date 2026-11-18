#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FirstPositions(u64);

impl FirstPositions {
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
        Self::reaches(position) && self.0 & (1 << position) != 0
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn positions(self) -> impl Iterator<Item = usize> {
        (0..Self::COUNT).filter(move |position| self.0 & (1 << position) != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::FirstPositions;

    #[test]
    fn a_position_past_the_first_word_is_never_contained() {
        let only = FirstPositions::of([3, 64, 70]);
        assert_eq!(only.positions().collect::<Vec<_>>(), [3]);
        assert!(!only.contains(64) && !only.contains(4));
    }

    #[test]
    fn the_last_position_of_the_first_word_is_kept() {
        let last = FirstPositions::of([63, 64]);
        assert!(last.contains(63) && !last.contains(64));
        assert_eq!(last.positions().collect::<Vec<_>>(), [63]);
    }

    #[test]
    fn a_union_keeps_the_positions_of_both() {
        let left = FirstPositions::of([1, 2]);
        let right = FirstPositions::of([2, 5]);
        assert_eq!(left.union(right), FirstPositions::of([1, 2, 5]));
    }
}

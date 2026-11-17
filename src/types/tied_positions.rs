use super::first_positions::FirstPositionsOnly;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TiedPositions {
    low: FirstPositionsOnly,
    high: Option<Box<HighPositions>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct HighPositions(Vec<usize>);

impl TiedPositions {
    pub fn of(positions: impl IntoIterator<Item = usize>) -> Self {
        positions
            .into_iter()
            .fold(Self::default(), |mut tied, position| {
                tied.insert(position);
                tied
            })
    }

    pub fn contains(&self, position: usize) -> bool {
        match FirstPositionsOnly::reaches(position) {
            true => self.low.contains(position),
            false => self
                .high
                .as_ref()
                .is_some_and(|high| high.0.contains(&position)),
        }
    }

    pub fn union(&self, other: &Self) -> Self {
        let mut union = Self {
            low: self.low.union(other.low),
            high: self.high.clone(),
        };
        for position in other.high.iter().flat_map(|high| high.0.iter().copied()) {
            union.insert(position);
        }
        union
    }

    pub fn positions(&self) -> impl Iterator<Item = usize> + '_ {
        self.low
            .positions()
            .chain(self.high.iter().flat_map(|high| high.0.iter().copied()))
    }

    fn insert(&mut self, position: usize) {
        match FirstPositionsOnly::reaches(position) {
            true => self.low = self.low.union(FirstPositionsOnly::of([position])),
            false => {
                let high = &mut self.high.get_or_insert_with(Box::default).0;
                if let Err(at) = high.binary_search(&position) {
                    high.insert(at, position);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TiedPositions;

    #[test]
    fn positions_below_and_above_a_word_are_kept_in_order() {
        let tied = TiedPositions::of([70, 3, 65, 3]);
        assert_eq!(tied.positions().collect::<Vec<_>>(), [3, 65, 70]);
        assert!(tied.contains(65) && tied.contains(3) && !tied.contains(4));
    }

    #[test]
    fn a_union_keeps_the_positions_of_both() {
        let union = TiedPositions::of([1, 80]).union(&TiedPositions::of([2]));
        assert_eq!(union.positions().collect::<Vec<_>>(), [1, 2, 80]);
    }
}

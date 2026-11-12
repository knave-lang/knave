pub struct SeededRandom {
    state: u64,
}

impl SeededRandom {
    const GOLDEN_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;
    const FIRST_MIX: u64 = 0xBF58_476D_1CE4_E5B9;
    const SECOND_MIX: u64 = 0x94D0_49BB_1331_11EB;
    const PERCENT: u64 = 100;

    pub fn starting_at(state: u64) -> Self {
        Self { state }
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound.max(1) as u64) as usize
    }

    pub fn chance(&mut self, percent: u64) -> bool {
        self.next() % Self::PERCENT < percent
    }

    pub fn pick<T: Copy>(&mut self, items: &[T]) -> Option<T> {
        (!items.is_empty()).then(|| items[self.below(items.len())])
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(Self::GOLDEN_GAMMA);
        let mut mixed = self.state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(Self::FIRST_MIX);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(Self::SECOND_MIX);
        mixed ^ (mixed >> 31)
    }
}

#[cfg(test)]
mod tests {
    use super::SeededRandom;

    #[test]
    fn one_seed_gives_one_sequence() {
        let mut first = SeededRandom::starting_at(7);
        let mut second = SeededRandom::starting_at(7);
        let drawn: Vec<usize> = (0..5).map(|_| first.below(1000)).collect();
        let again: Vec<usize> = (0..5).map(|_| second.below(1000)).collect();
        assert_eq!(drawn, again);
        assert_ne!(SeededRandom::starting_at(8).below(1000), drawn[0]);
    }

    #[test]
    fn draws_stay_below_their_bound() {
        let mut random = SeededRandom::starting_at(1);
        assert!((0..100).all(|_| random.below(3) < 3));
        assert_eq!(random.pick::<u8>(&[]), None);
    }
}

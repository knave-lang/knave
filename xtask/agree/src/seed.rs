use std::fmt;
use std::ops::Range;
use std::str::FromStr;

use crate::seeded_random::SeededRandom;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Seed(u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeedRange {
    seeds: Range<u64>,
}

impl Seed {
    const PREFIX: &'static str = "seed-";

    pub fn random(self) -> SeededRandom {
        SeededRandom::starting_at(self.0)
    }
}

impl FromStr for Seed {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        text.strip_prefix(Self::PREFIX)
            .and_then(|number| number.parse().ok())
            .map(Self)
            .ok_or_else(|| format!("`{text}` does not name a seed"))
    }
}

impl From<u64> for Seed {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl fmt::Display for Seed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}", Self::PREFIX, self.0)
    }
}

impl SeedRange {
    const SEPARATOR: &'static str = "..";

    pub fn new(seeds: Range<u64>) -> Self {
        Self { seeds }
    }

    pub fn contains(&self, seed: Seed) -> bool {
        self.seeds.contains(&seed.0)
    }

    pub fn seeds(&self) -> impl Iterator<Item = Seed> + '_ {
        self.seeds.clone().map(Seed)
    }
}

impl FromStr for SeedRange {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        let (first, end) = text
            .split_once(Self::SEPARATOR)
            .ok_or_else(|| format!("`{text}` is not a range such as `0..200`"))?;
        let first: u64 = first
            .parse()
            .map_err(|_| format!("`{first}` is not a seed"))?;
        let end: u64 = end.parse().map_err(|_| format!("`{end}` is not a seed"))?;
        Ok(Self::new(first..end))
    }
}

#[cfg(test)]
mod tests {
    use super::{Seed, SeedRange};

    #[test]
    fn a_seed_names_itself() {
        assert_eq!(Seed::from(3).to_string(), "seed-3");
    }

    #[test]
    fn reads_a_seed_range_and_lists_its_seeds() {
        let range: SeedRange = "2..4".parse().expect("a seed range");
        let seeds: Vec<Seed> = range.seeds().collect();
        assert_eq!(seeds, [Seed::from(2), Seed::from(3)]);
    }

    #[test]
    fn reads_back_the_name_it_writes() {
        assert_eq!("seed-12".parse::<Seed>(), Ok(Seed::from(12)));
        assert!("main".parse::<Seed>().is_err());
        assert!(SeedRange::new(10..13).contains(Seed::from(12)));
        assert!(!SeedRange::new(10..12).contains(Seed::from(12)));
    }

    #[test]
    fn refuses_text_that_is_not_a_seed_range() {
        assert!("200".parse::<SeedRange>().is_err());
        assert!("a..2".parse::<SeedRange>().is_err());
        assert!("0..b".parse::<SeedRange>().is_err());
    }
}

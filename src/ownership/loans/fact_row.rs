const FACTS_PER_WORD: usize = u64::BITS as usize;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FactRow {
    words: Vec<u64>,
}

impl FactRow {
    pub fn sized(facts: usize) -> Self {
        Self {
            words: vec![0; facts.div_ceil(FACTS_PER_WORD)],
        }
    }

    pub fn contains(&self, fact: usize) -> bool {
        self.words
            .get(fact / FACTS_PER_WORD)
            .is_some_and(|word| word & Self::bit(fact) != 0)
    }

    pub fn set(&mut self, fact: usize) {
        self.words[fact / FACTS_PER_WORD] |= Self::bit(fact);
    }

    pub fn clear(&mut self, fact: usize) {
        self.words[fact / FACTS_PER_WORD] &= !Self::bit(fact);
    }

    pub fn absorb(&mut self, other: &Self) -> bool {
        let mut grew = false;
        for (word, fact) in self.words.iter_mut().zip(&other.words) {
            let joined = *word | fact;
            grew |= joined != *word;
            *word = joined;
        }
        grew
    }

    pub fn meet(&mut self, other: &Self) {
        for (word, kept) in self.words.iter_mut().zip(&other.words) {
            *word &= kept;
        }
    }

    fn bit(fact: usize) -> u64 {
        1 << (fact % FACTS_PER_WORD)
    }
}

#[cfg(test)]
mod tests {
    use super::FactRow;

    #[test]
    fn a_row_absorbs_facts_and_reports_growth_only_once() {
        let mut row = FactRow::sized(130);
        let mut facts = FactRow::sized(130);
        facts.set(129);
        facts.set(3);
        facts.clear(3);
        assert!(row.absorb(&facts));
        assert!(!row.absorb(&facts));
        assert!(row.contains(129));
        assert!(!row.contains(3));
        assert!(!row.contains(500));
    }

    #[test]
    fn a_row_met_with_another_keeps_only_their_shared_facts() {
        let mut row = FactRow::sized(70);
        row.set(1);
        row.set(65);
        let mut kept = FactRow::sized(70);
        kept.set(65);
        row.meet(&kept);
        assert!(!row.contains(1));
        assert!(row.contains(65));
    }
}

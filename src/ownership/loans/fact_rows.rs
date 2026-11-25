const FACTS_PER_WORD: usize = u64::BITS as usize;

pub struct FactRows {
    width: usize,
    words: Vec<u64>,
}

impl FactRows {
    pub fn new(rows: usize, facts: usize) -> Self {
        let width = facts.div_ceil(FACTS_PER_WORD);
        Self {
            width,
            words: vec![0; rows * width],
        }
    }

    pub fn contains(&self, row: usize, fact: usize) -> bool {
        self.row(row)
            .get(fact / FACTS_PER_WORD)
            .is_some_and(|word| word & Self::bit(fact) != 0)
    }

    pub fn copy_row(&self, row: usize, into: &mut Vec<u64>) {
        into.clear();
        into.extend_from_slice(self.row(row));
    }

    pub fn absorb(&mut self, row: usize, facts: &[u64]) -> bool {
        let start = row * self.width;
        let mut grew = false;
        for (word, fact) in self.words[start..start + self.width].iter_mut().zip(facts) {
            let joined = *word | fact;
            grew |= joined != *word;
            *word = joined;
        }
        grew
    }

    pub fn set(facts: &mut [u64], fact: usize) {
        facts[fact / FACTS_PER_WORD] |= Self::bit(fact);
    }

    pub fn clear(facts: &mut [u64], fact: usize) {
        facts[fact / FACTS_PER_WORD] &= !Self::bit(fact);
    }

    fn row(&self, row: usize) -> &[u64] {
        let start = row * self.width;
        self.words
            .get(start..start + self.width)
            .unwrap_or_default()
    }

    fn bit(fact: usize) -> u64 {
        1 << (fact % FACTS_PER_WORD)
    }
}

#[cfg(test)]
mod tests {
    use super::FactRows;

    #[test]
    fn a_row_absorbs_facts_and_reports_growth_only_once() {
        let mut rows = FactRows::new(2, 130);
        let mut facts = vec![0; 3];
        FactRows::set(&mut facts, 129);
        FactRows::set(&mut facts, 3);
        FactRows::clear(&mut facts, 3);
        assert!(rows.absorb(1, &facts));
        assert!(!rows.absorb(1, &facts));
        assert!(rows.contains(1, 129));
        assert!(!rows.contains(1, 3));
        assert!(!rows.contains(0, 129));
        assert!(!rows.contains(5, 0));
    }
}

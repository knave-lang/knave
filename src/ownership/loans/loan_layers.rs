use super::loan_depth::LoanDepth;
use super::loan_id::LoanId;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct LoanLayers {
    loans: Vec<(LoanDepth, LoanId)>,
}

impl LoanLayers {
    pub fn insert(&mut self, depth: LoanDepth, loan: LoanId) {
        if let Err(position) = self.loans.binary_search(&(depth, loan)) {
            self.loans.insert(position, (depth, loan));
        }
    }

    pub fn holds(&self, depth: LoanDepth, loan: LoanId) -> bool {
        self.loans.binary_search(&(depth, loan)).is_ok()
    }

    pub fn loans(&self) -> impl Iterator<Item = (LoanDepth, LoanId)> + '_ {
        self.loans.iter().copied()
    }

    pub fn referent(&self) -> Self {
        self.shifted(-1)
    }

    pub fn within(&self) -> Self {
        self.shifted(1)
    }

    fn shifted(&self, offset: isize) -> Self {
        self.loans()
            .filter_map(|(depth, loan)| Some((depth.shifted(offset)?, loan)))
            .collect()
    }
}

impl Extend<(LoanDepth, LoanId)> for LoanLayers {
    fn extend<T: IntoIterator<Item = (LoanDepth, LoanId)>>(&mut self, loans: T) {
        for (depth, loan) in loans {
            self.insert(depth, loan);
        }
    }
}

impl FromIterator<(LoanDepth, LoanId)> for LoanLayers {
    fn from_iter<T: IntoIterator<Item = (LoanDepth, LoanId)>>(loans: T) -> Self {
        let mut layers = Self::default();
        layers.extend(loans);
        layers
    }
}

#[cfg(test)]
mod tests {
    use super::LoanLayers;
    use crate::ownership::loans::loan_depth::LoanDepth;
    use crate::ownership::loans::loan_id::LoanId;

    #[test]
    fn a_loan_held_within_moves_one_layer_deeper_and_back_under_its_referent() {
        let top: LoanLayers = [(LoanDepth::TOP, LoanId::new(1))].into_iter().collect();
        let within = top.within();
        assert!(within.holds(LoanDepth::at(1), LoanId::new(1)));
        assert_eq!(within.referent(), top);
        assert_eq!(top.referent().loans().count(), 0);
    }

    #[test]
    fn a_loan_held_past_the_deepest_layer_is_kept_at_the_deepest() {
        let deepest: LoanLayers = [(LoanDepth::DEEPEST, LoanId::new(1))].into_iter().collect();
        assert_eq!(deepest.within(), deepest);
        assert_eq!(deepest.referent(), deepest);
    }
}

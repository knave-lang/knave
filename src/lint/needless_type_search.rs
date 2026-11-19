use super::removal::Removal;
use super::removal_trial::{RemovalTrial, TrialVerdict};
use crate::core::BindingId;

pub struct NeedlessTypeSearch<'t, Trial: RemovalTrial> {
    trial: &'t mut Trial,
    removal: Removal<Trial::Kept>,
}

impl<'t, Trial: RemovalTrial> NeedlessTypeSearch<'t, Trial> {
    pub fn new(trial: &'t mut Trial) -> Self {
        Self {
            trial,
            removal: Removal::default(),
        }
    }

    pub fn jointly_removable(mut self, declared: &[BindingId]) -> Removal<Trial::Kept> {
        let blamed = self.settle_blaming(declared.to_vec());
        for function in blamed {
            self.settle_alone(function);
        }
        self.removal
    }

    fn settle_alone(&mut self, function: BindingId) {
        if let TrialVerdict::Kept(kept) = self.verdict_without(&[function]) {
            self.accept(&[function], kept);
        }
    }

    fn settle_blaming(&mut self, mut group: Vec<BindingId>) -> Vec<BindingId> {
        let mut blamed = Vec::new();
        while !group.is_empty() {
            match self.verdict_without(&group) {
                TrialVerdict::Kept(kept) => {
                    self.accept(&group, kept);
                    break;
                }
                TrialVerdict::Rejected(culprits) if Self::blames_any(&group, &culprits) => {
                    let (rest, in_group) = Self::without_culprits(group, &culprits);
                    group = rest;
                    blamed.extend(in_group);
                }
                TrialVerdict::Rejected(_) => {
                    blamed.extend(self.settle_halves(&group));
                    break;
                }
            }
        }
        blamed
    }

    fn settle_halves(&mut self, group: &[BindingId]) -> Vec<BindingId> {
        if group.len() < 2 {
            return Vec::new();
        }
        let (front, back) = group.split_at(group.len() / 2);
        let mut blamed = self.settle_blaming(front.to_vec());
        blamed.extend(self.settle_blaming(back.to_vec()));
        blamed
    }

    fn blames_any(group: &[BindingId], culprits: &[BindingId]) -> bool {
        culprits.iter().any(|function| group.contains(function))
    }

    fn without_culprits(
        group: Vec<BindingId>,
        culprits: &[BindingId],
    ) -> (Vec<BindingId>, Vec<BindingId>) {
        group
            .into_iter()
            .partition(|function| !culprits.contains(function))
    }

    fn accept(&mut self, group: &[BindingId], kept: Trial::Kept) {
        self.removal.removable.extend_from_slice(group);
        self.removal.kept = Some(kept);
    }

    fn verdict_without(&mut self, group: &[BindingId]) -> TrialVerdict<Trial::Kept> {
        let removed: Vec<BindingId> = self
            .removal
            .removable
            .iter()
            .chain(group)
            .copied()
            .collect();
        self.trial.verdict(&removed)
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashSet;

    use super::NeedlessTypeSearch;
    use crate::core::BindingId;
    use crate::lint::removal_trial::{RemovalTrial, TrialVerdict};

    #[derive(Clone, Copy)]
    enum Blame {
        NeededDeclarations,
        Nothing,
        EveryRemoval,
    }

    struct NeededDeclarations {
        needed_alone: FxHashSet<BindingId>,
        needed_together: Option<(BindingId, BindingId)>,
        blame: Blame,
        removals: Vec<Vec<BindingId>>,
    }

    impl RemovalTrial for NeededDeclarations {
        type Kept = ();

        fn verdict(&mut self, removed: &[BindingId]) -> TrialVerdict<()> {
            self.removals.push(removed.to_vec());
            let needed = self.needed_among(removed);
            if needed.is_empty() {
                return TrialVerdict::Kept(());
            }
            TrialVerdict::Rejected(match self.blame {
                Blame::NeededDeclarations => needed,
                Blame::Nothing => Vec::new(),
                Blame::EveryRemoval => removed.to_vec(),
            })
        }
    }

    impl NeededDeclarations {
        const DECLARED: u32 = 6;

        fn new(blame: Blame) -> Self {
            Self {
                needed_alone: FxHashSet::default(),
                needed_together: None,
                blame,
                removals: Vec::new(),
            }
        }

        fn needing(mut self, function: u32) -> Self {
            self.needed_alone.insert(BindingId::from_index(function));
            self
        }

        fn needing_one_of(mut self, first: u32, second: u32) -> Self {
            self.needed_together =
                Some((BindingId::from_index(first), BindingId::from_index(second)));
            self
        }

        fn needed_among(&self, removed: &[BindingId]) -> Vec<BindingId> {
            let mut needed: Vec<BindingId> = removed
                .iter()
                .copied()
                .filter(|function| self.needed_alone.contains(function))
                .collect();
            if let Some((first, second)) = self.needed_together
                && removed.contains(&first)
                && removed.contains(&second)
            {
                needed.push(second);
            }
            needed
        }

        fn declared() -> Vec<BindingId> {
            (0..Self::DECLARED).map(BindingId::from_index).collect()
        }

        fn search(&mut self) -> (Vec<BindingId>, usize) {
            let removal = NeedlessTypeSearch::new(self).jointly_removable(&Self::declared());
            (removal.removable, self.removals.len())
        }

        fn keeps_output_without(&self, removable: &[BindingId]) -> bool {
            self.needed_among(removable).is_empty()
        }
    }

    #[test]
    fn the_first_trial_removes_every_declaration() {
        let mut trial = NeededDeclarations::new(Blame::NeededDeclarations).needing(2);
        trial.search();
        assert_eq!(trial.removals[0], NeededDeclarations::declared());
    }

    #[test]
    fn every_declaration_is_removable_in_one_trial_when_nothing_needs_one() {
        let mut trial = NeededDeclarations::new(Blame::NeededDeclarations);
        assert_eq!(trial.search(), (NeededDeclarations::declared(), 1));
    }

    #[test]
    fn two_declarations_that_settle_each_other_are_not_both_removable() {
        for blame in [
            Blame::NeededDeclarations,
            Blame::Nothing,
            Blame::EveryRemoval,
        ] {
            let mut trial = NeededDeclarations::new(blame).needing_one_of(1, 4);
            let (removable, _) = trial.search();
            assert!(trial.keeps_output_without(&removable));
            assert_eq!(removable.len(), 5);
        }
    }

    #[test]
    fn a_blamed_declaration_is_tried_alone_after_the_rest_is_removed_together() {
        let mut trial = NeededDeclarations::new(Blame::NeededDeclarations).needing(2);
        let (removable, trials) = trial.search();
        let without_the_blamed: Vec<BindingId> = NeededDeclarations::declared()
            .into_iter()
            .filter(|function| *function != BindingId::from_index(2))
            .collect();
        assert_eq!(trial.removals[1], without_the_blamed);
        assert_eq!((removable, trials), (without_the_blamed, 3));
    }

    #[test]
    fn a_blamed_declaration_is_removable_alone_when_its_partner_stays() {
        let mut trial = NeededDeclarations::new(Blame::NeededDeclarations)
            .needing(3)
            .needing_one_of(3, 2);
        let (removable, _) = trial.search();
        assert!(trial.keeps_output_without(&removable));
        assert_eq!(
            removable,
            [0, 1, 4, 5, 2].map(BindingId::from_index).to_vec()
        );
    }

    #[test]
    fn a_rejection_that_blames_nothing_halves_the_group() {
        let mut trial = NeededDeclarations::new(Blame::Nothing).needing(4);
        let (removable, trials) = trial.search();
        let front: Vec<BindingId> = (0..3).map(BindingId::from_index).collect();
        assert_eq!(trial.removals[1], front);
        assert!(!removable.contains(&BindingId::from_index(4)));
        assert_eq!((removable.len(), trials), (5, 7));
    }

    #[test]
    fn a_rejection_that_blames_the_whole_group_tries_each_declaration_alone() {
        let mut trial = NeededDeclarations::new(Blame::NeededDeclarations);
        for function in 0..NeededDeclarations::DECLARED {
            trial = trial.needing(function);
        }
        let (removable, trials) = trial.search();
        assert!(removable.is_empty());
        assert_eq!(trials, NeededDeclarations::DECLARED as usize + 1);
    }

    #[test]
    fn the_search_ends_within_two_trials_per_declaration_when_every_trial_fails() {
        for blame in [Blame::Nothing, Blame::EveryRemoval] {
            let mut trial = NeededDeclarations::new(blame);
            for function in 0..NeededDeclarations::DECLARED {
                trial = trial.needing(function);
            }
            let (removable, trials) = trial.search();
            assert!(removable.is_empty());
            assert!(trials < 2 * NeededDeclarations::DECLARED as usize);
        }
    }
}

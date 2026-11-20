use super::declaration_key::DeclarationKey;
use super::keyed_candidates::KeyedCandidates;
use super::linted_program::LintedProgram;
use super::program_position::ProgramPosition;
use super::program_trials::ProgramTrials;
use super::removal::Removal;

pub struct SettledRemovals<Kept> {
    pub removals: Vec<Removal<Kept>>,
    pub removable: Vec<DeclarationKey>,
}

pub struct JointRemoval;

impl JointRemoval {
    pub fn settle<Trials: ProgramTrials>(
        trials: &mut Trials,
        programs: &mut [LintedProgram<'_>],
        keyed: &[KeyedCandidates],
    ) -> SettledRemovals<Trials::Kept> {
        let mut tried = KeyedCandidates::keys_to_try(keyed);
        loop {
            let removals: Vec<Removal<Trials::Kept>> = programs
                .iter_mut()
                .zip(keyed)
                .enumerate()
                .map(|(position, (linted, keyed))| {
                    let tried_here = keyed.tried_among(&tried);
                    trials.removal(ProgramPosition::nth(position), linted, tried_here)
                })
                .collect();
            let kept: Vec<DeclarationKey> = tried
                .iter()
                .filter(|key| {
                    !keyed
                        .iter()
                        .zip(&removals)
                        .any(|(keyed, removal)| keyed.refuses(&removal.removable, key))
                })
                .cloned()
                .collect();
            let settled = !keyed
                .iter()
                .zip(&removals)
                .any(|(keyed, removal)| keyed.removes_any_but(&removal.removable, &kept));
            if settled {
                return SettledRemovals {
                    removals,
                    removable: kept,
                };
            }
            tried = kept;
        }
    }
}

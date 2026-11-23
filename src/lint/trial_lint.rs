use rustc_hash::FxHashSet;

use super::declaration_key::DeclarationKey;
use super::joint_removal::JointRemoval;
use super::keyed_candidates::{KeyedCandidate, KeyedCandidates};
use super::linted_program::LintedProgram;
use super::module_alone_search::ModuleAloneSearch;
use super::program_candidates::ProgramCandidates;
use super::program_position::ProgramPosition;
use super::program_run::ProgramRun;
use super::program_trials::ProgramTrials;
use super::removal::Removal;
use super::rule_declarations::RuleDeclarations;
use super::rule_search::RuleSearch;
use super::trial_outcome::TrialOutcome;
use super::trial_rule::TrialRule;
use super::written_program::WrittenProgram;
use crate::core::BindingId;
use crate::expand::Program;
use crate::project::LintLevel;

pub struct TrialLint<'r, Run: ProgramRun, Rule: TrialRule> {
    run: &'r mut Run,
    rule: &'r Rule,
}

impl<'r, Run: ProgramRun, Rule: TrialRule> TrialLint<'r, Run, Rule> {
    pub fn new(run: &'r mut Run, rule: &'r Rule) -> Self {
        Self { run, rule }
    }

    pub fn candidates(
        &self,
        level: Option<LintLevel>,
        programs: &[LintedProgram<'_>],
    ) -> ProgramCandidates<Rule::Declaration> {
        let runs = level != Some(LintLevel::Allow);
        ProgramCandidates::new(
            programs
                .iter()
                .map(|linted| {
                    if runs {
                        self.rule.candidates(linted)
                    } else {
                        Vec::new()
                    }
                })
                .collect(),
        )
    }

    pub fn outcome(
        &mut self,
        candidates: ProgramCandidates<Rule::Declaration>,
        programs: &mut [LintedProgram<'_>],
    ) -> TrialOutcome {
        let candidates = candidates.into_lists();
        let keyed: Vec<KeyedCandidates> = programs
            .iter()
            .zip(&candidates)
            .map(|(linted, candidates)| self.keyed(linted.program, candidates))
            .collect();
        let settled = JointRemoval::settle(self, programs, &keyed);
        let mut warned = FxHashSet::default();
        let mut outcome = TrialOutcome::default();
        for (((linted, candidates), keyed), removal) in programs
            .iter()
            .zip(&candidates)
            .zip(&keyed)
            .zip(settled.removals)
        {
            let first_here = Removal {
                removable: keyed.first_warned(&settled.removable, &mut warned),
                kept: removal.kept,
            };
            let warnings = self.rule.warnings(linted.program, candidates, first_here);
            outcome.warnings.extend(warnings);
            outcome.removed.push(keyed.tried_among(&settled.removable));
        }
        outcome
    }

    fn keyed(&self, program: &Program, candidates: &[Rule::Declaration]) -> KeyedCandidates {
        let written = WrittenProgram::new(program);
        KeyedCandidates::new(
            candidates
                .iter()
                .map(|declaration| {
                    let candidate = self.rule.candidate(declaration);
                    let file = self.run.source_path(candidate.span.source());
                    KeyedCandidate {
                        binding: candidate.binding,
                        key: DeclarationKey::new(
                            file,
                            candidate.module,
                            written.name(candidate.binding),
                        ),
                        tried: candidate.tried,
                    }
                })
                .collect(),
        )
    }

    fn accepted_by_modules(
        &mut self,
        position: ProgramPosition,
        linted: &LintedProgram<'_>,
        removal: &Removal<Rule::Kept>,
    ) -> Vec<BindingId> {
        let checks = self.rule.module_checks(linted, removal);
        let unrefused: Vec<BindingId> = removal
            .removable
            .iter()
            .copied()
            .filter(|binding| !checks.refused.contains(binding))
            .collect();
        let libraries = RuleDeclarations::new(self.rule, linted.program)
            .library_declarations(&checks.checked_alone);
        ModuleAloneSearch::new(&mut *self.run, self.rule, position).accepted(
            linted.program,
            libraries,
            &unrefused,
        )
    }
}

impl<Run: ProgramRun, Rule: TrialRule> ProgramTrials for TrialLint<'_, Run, Rule> {
    type Kept = Rule::Kept;

    fn removal(
        &mut self,
        position: ProgramPosition,
        linted: &mut LintedProgram<'_>,
        mut tried: Vec<BindingId>,
    ) -> Removal<Rule::Kept> {
        loop {
            let removal = RuleSearch::new(&mut *self.run, self.rule).jointly_removable(
                linted.program,
                linted.declared,
                &tried,
            );
            let accepted = self.accepted_by_modules(position, linted, &removal);
            if accepted.len() == removal.removable.len() {
                return removal;
            }
            tried = accepted;
        }
    }
}

use rustc_hash::FxHashSet;

use super::declaration_key::DeclarationKey;
use super::declared_functions::DeclaredFunctions;
use super::function_declaration::FunctionDeclaration;
use super::linted_program::LintedProgram;
use super::module_alone_search::ModuleAloneSearch;
use super::needless_type_search::NeedlessTypeSearch;
use super::needless_type_trial::{ByValueRewrites, NeedlessTypeTrial};
use super::needless_type_warnings::NeedlessTypeWarnings;
use super::program_position::ProgramPosition;
use super::program_run::ProgramRun;
use super::removal::Removal;
use crate::core::BindingId;
use crate::diagnostic::Diagnostics;
use crate::expand::Program;
use crate::project::LintLevel;
use crate::types::TypedProgram;

pub struct NeedlessTypeLint<'r, Run: ProgramRun> {
    run: &'r mut Run,
}

struct KeyedCandidates {
    candidates: Vec<FunctionDeclaration>,
    keys: Vec<DeclarationKey>,
}

impl KeyedCandidates {
    fn tried_among(&self, tried: &[DeclarationKey]) -> Vec<BindingId> {
        self.candidates
            .iter()
            .zip(&self.keys)
            .filter(|(_, key)| tried.contains(key))
            .map(|(declaration, _)| declaration.function)
            .collect()
    }
}

struct ProgramRemoval<'k> {
    keyed: &'k KeyedCandidates,
    removal: Removal<ByValueRewrites>,
}

impl ProgramRemoval<'_> {
    fn refuses(&self, key: &DeclarationKey) -> bool {
        self.keyed()
            .any(|(candidate, removed)| candidate == key && !removed)
    }

    fn removes_any_but(&self, kept: &[DeclarationKey]) -> bool {
        self.keyed()
            .any(|(candidate, removed)| removed && !kept.contains(candidate))
    }

    fn keyed(&self) -> impl Iterator<Item = (&DeclarationKey, bool)> {
        self.keyed
            .keys
            .iter()
            .zip(&self.keyed.candidates)
            .map(|(key, declaration)| (key, self.removal.removable.contains(&declaration.function)))
    }
}

impl<'r, Run: ProgramRun> NeedlessTypeLint<'r, Run> {
    pub fn new(run: &'r mut Run) -> Self {
        Self { run }
    }

    pub fn warnings(
        &mut self,
        level: Option<LintLevel>,
        programs: &mut [LintedProgram<'_>],
    ) -> Diagnostics {
        if level == Some(LintLevel::Allow) {
            return Diagnostics::default();
        }
        let keyed = self.keyed(programs);
        let mut tried = Self::tried_keys(&keyed);
        loop {
            let removals = self.removals(programs, &keyed, &tried);
            let kept: Vec<DeclarationKey> = tried
                .iter()
                .filter(|key| !removals.iter().any(|removal| removal.refuses(key)))
                .cloned()
                .collect();
            if !removals
                .iter()
                .any(|removal| removal.removes_any_but(&kept))
            {
                return Self::warned_once(programs, removals, &kept);
            }
            tried = kept;
        }
    }

    fn keyed(&self, programs: &[LintedProgram<'_>]) -> Vec<KeyedCandidates> {
        programs
            .iter()
            .map(|linted| {
                let declared = DeclaredFunctions::new(linted.program);
                let candidates = declared.candidates();
                let keys = candidates
                    .iter()
                    .map(|declaration| {
                        let file = self.run.source_path(declaration.span.source());
                        declared.key(declaration, file)
                    })
                    .collect();
                KeyedCandidates { candidates, keys }
            })
            .collect()
    }

    fn tried_keys(keyed: &[KeyedCandidates]) -> Vec<DeclarationKey> {
        let mut tried = Vec::new();
        for program in keyed {
            for (declaration, key) in program.candidates.iter().zip(&program.keys) {
                if declaration.needs_a_trial() && !tried.contains(key) {
                    tried.push(key.clone());
                }
            }
        }
        tried
    }

    fn removals<'k>(
        &mut self,
        programs: &mut [LintedProgram<'_>],
        keyed: &'k [KeyedCandidates],
        tried: &[DeclarationKey],
    ) -> Vec<ProgramRemoval<'k>> {
        programs
            .iter_mut()
            .zip(keyed)
            .enumerate()
            .map(|(position, (linted, keyed))| {
                let tried_here = keyed.tried_among(tried);
                let removal = self.removal(ProgramPosition::nth(position), linted, tried_here);
                ProgramRemoval { keyed, removal }
            })
            .collect()
    }

    fn warned_once(
        programs: &[LintedProgram<'_>],
        removals: Vec<ProgramRemoval<'_>>,
        removable: &[DeclarationKey],
    ) -> Diagnostics {
        let mut warned: FxHashSet<DeclarationKey> = FxHashSet::default();
        let mut warnings = Diagnostics::default();
        for (linted, program_removal) in programs.iter().zip(removals) {
            let keyed = program_removal.keyed;
            let first_here: Vec<BindingId> = keyed
                .keys
                .iter()
                .zip(&keyed.candidates)
                .filter(|(key, _)| removable.contains(key) && warned.insert((*key).clone()))
                .map(|(_, declaration)| declaration.function)
                .collect();
            let by_value = program_removal.removal.kept.unwrap_or_default();
            warnings.extend(NeedlessTypeWarnings::new(linted.program).of(
                &keyed.candidates,
                &first_here,
                &by_value,
            ));
        }
        warnings
    }

    fn removal(
        &mut self,
        position: ProgramPosition,
        linted: &mut LintedProgram<'_>,
        mut tried: Vec<BindingId>,
    ) -> Removal<ByValueRewrites> {
        loop {
            let removal = self.program_search(linted.program, linted.declared, &tried);
            let accepted = self.accepted_by_modules(position, linted.program, &removal.removable);
            if accepted.len() == removal.removable.len() {
                return removal;
            }
            tried = accepted;
        }
    }

    fn program_search(
        &mut self,
        program: &mut Program,
        declared: &TypedProgram,
        tried: &[BindingId],
    ) -> Removal<ByValueRewrites> {
        if tried.is_empty() {
            return Removal::default();
        }
        let mut trial = NeedlessTypeTrial::new(&mut *self.run, program, declared);
        NeedlessTypeSearch::new(&mut trial).jointly_removable(tried)
    }

    fn accepted_by_modules(
        &mut self,
        position: ProgramPosition,
        program: &Program,
        removable: &[BindingId],
    ) -> Vec<BindingId> {
        let declared = DeclaredFunctions::new(program);
        let mut refused = Vec::new();
        for library in declared.library_declarations(removable) {
            let accepted =
                ModuleAloneSearch::new(&mut *self.run, position).removable_names(&library);
            let refused_names: Vec<String> = library
                .names
                .into_iter()
                .filter(|name| !accepted.contains(name))
                .collect();
            refused.extend(
                declared
                    .named_in(&library.module, &refused_names)
                    .iter()
                    .map(|declaration| declaration.function),
            );
        }
        removable
            .iter()
            .copied()
            .filter(|function| !refused.contains(function))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::lint::needless_type_fixture::{Emission, NeedlessTypeFixture, ScriptedRun};
    use crate::project::LintLevel;

    const TWICE: &str = "(type twice (fn (int) int))

(defn twice (x) (* 2 x))

(defn main () (println (twice 4)))
";

    const DISTANCE: &str = "(defstruct point :x int :y int)

(type distance (fn ((ref point) (ref point)) int))

(defn distance (from to) (+ (- (:x to) (:x from)) (- (:y to) (:y from))))

(defn main () (println (distance (->point :x 1 :y 2) (->point :x 4 :y 6))))
";

    const FIRST_WORD: &str = "(type first-word (where ('a) (fn ((ref 'a str)) (ref 'a str))))

(defn first-word (text) text)

(defn main () (println (first-word \"knave\")))
";

    #[test]
    fn a_declaration_that_inference_gives_warns() {
        let titles = NeedlessTypeFixture::expand(TWICE)
            .warning_titles(ScriptedRun::emitting(Emission::Unchanged));
        assert_eq!(titles, ["`twice` does not need its `type` declaration"]);
    }

    #[test]
    fn an_allowed_level_runs_no_trial() {
        let (titles, checks) = NeedlessTypeFixture::expand(TWICE).warning_titles_at(
            Some(LintLevel::Allow),
            ScriptedRun::emitting(Emission::Unchanged),
        );
        assert_eq!((titles.len(), checks), (0, 0));
    }

    #[test]
    fn a_denied_level_runs_the_trials() {
        let (titles, checks) = NeedlessTypeFixture::expand(TWICE).warning_titles_at(
            Some(LintLevel::Deny),
            ScriptedRun::emitting(Emission::Unchanged),
        );
        assert_eq!((titles.len(), checks), (1, 1));
    }

    #[test]
    fn a_declaration_that_writes_a_lifetime_runs_no_trial() {
        let (titles, checks) = NeedlessTypeFixture::expand(FIRST_WORD).warning_titles_at(
            Some(LintLevel::Warn),
            ScriptedRun::emitting(Emission::Unchanged),
        );
        assert_eq!((titles.len(), checks), (0, 0));
    }

    #[test]
    fn a_declaration_that_two_programs_hold_warns_once() {
        let titles = NeedlessTypeFixture::expand(TWICE)
            .warning_titles_in_two_programs(TWICE, ScriptedRun::emitting(Emission::Unchanged));
        assert_eq!(titles, ["`twice` does not need its `type` declaration"]);
    }

    #[test]
    fn a_declaration_whose_removal_changes_only_the_rust_stays_silent() {
        let titles = NeedlessTypeFixture::expand(TWICE)
            .warning_titles(ScriptedRun::emitting(Emission::DifferentEachTime));
        assert!(titles.is_empty(), "{titles:?}");
    }

    #[test]
    fn a_by_value_declaration_warns_when_its_rewrite_checks() {
        let titles = NeedlessTypeFixture::expand(DISTANCE)
            .warning_titles(ScriptedRun::emitting(Emission::Unchanged));
        assert_eq!(titles, ["`distance` does not need its `type` declaration"]);
    }

    #[test]
    fn a_by_value_declaration_stays_when_its_rewrite_does_not_check() {
        let stripped_then_rewritten = 2;
        let run = ScriptedRun::emitting(Emission::Unchanged).failing_check(stripped_then_rewritten);
        let titles = NeedlessTypeFixture::expand(DISTANCE).warning_titles(run);
        assert!(titles.is_empty(), "{titles:?}");
    }

    #[test]
    fn an_exported_declaration_stays_silent_when_its_module_does_not_expand_alone() {
        let titles = NeedlessTypeFixture::expand_lint_fixture("exported_warns")
            .warning_titles(ScriptedRun::emitting(Emission::Unchanged));
        assert!(titles.is_empty(), "{titles:?}");
    }
}

use super::declared_functions::DeclaredFunctions;
use super::function_declaration::FunctionDeclaration;
use super::linted_program::LintedProgram;
use super::module_checks::ModuleChecks;
use super::needless_type_trial::{ByValueRewrites, NeedlessTypeTrial};
use super::needless_type_warnings::NeedlessTypeWarnings;
use super::program_run::ProgramRun;
use super::removal::Removal;
use super::trial_candidate::TrialCandidate;
use super::trial_rule::TrialRule;
use crate::core::{BindingId, CoreModule};
use crate::diagnostic::Diagnostics;
use crate::expand::Program;
use crate::types::TypedProgram;

pub struct NeedlessTypeRule;

impl TrialRule for NeedlessTypeRule {
    type Declaration = FunctionDeclaration;
    type Kept = ByValueRewrites;
    type Trial<'r, Run: ProgramRun + 'r> = NeedlessTypeTrial<'r, Run>;

    fn candidates(&self, linted: &LintedProgram<'_>) -> Vec<FunctionDeclaration> {
        DeclaredFunctions::new(linted.program).candidates()
    }

    fn candidate<'d>(&self, declaration: &'d FunctionDeclaration) -> TrialCandidate<'d> {
        TrialCandidate {
            binding: declaration.function,
            module: &declaration.module,
            span: declaration.span,
            tried: declaration.needs_a_trial(),
        }
    }

    fn declared_in(&self, program: &Program, module: &CoreModule) -> Vec<BindingId> {
        DeclaredFunctions::new(program)
            .declared_in(module)
            .iter()
            .map(|declaration| declaration.function)
            .collect()
    }

    fn trial<'r, Run: ProgramRun>(
        &self,
        run: &'r mut Run,
        program: &'r mut Program,
        declared: &'r TypedProgram,
    ) -> NeedlessTypeTrial<'r, Run> {
        NeedlessTypeTrial::new(run, program, declared)
    }

    fn module_checks(
        &self,
        _linted: &LintedProgram<'_>,
        removal: &Removal<ByValueRewrites>,
    ) -> ModuleChecks {
        ModuleChecks {
            refused: Vec::new(),
            checked_alone: removal.removable.clone(),
        }
    }

    fn warnings(
        &self,
        program: &Program,
        declarations: &[FunctionDeclaration],
        warned: Removal<ByValueRewrites>,
    ) -> Diagnostics {
        let by_value = warned.kept.unwrap_or_default();
        NeedlessTypeWarnings::new(program).of(declarations, &warned.removable, &by_value)
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

    const TWICE_AND_THRICE: &str = "(type twice (fn (int) int))

(defn twice (x) (* 2 x))

(type thrice (fn (int) int))

(defn thrice (x) (* 3 x))

(defn main () (println (thrice (twice 4))))
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
    fn a_declaration_that_held_only_beside_a_refused_one_stays_silent() {
        let jointly_in_the_first_then_twice_in_the_second = 2;
        let thrice_alone_in_the_first = 3;
        let run = ScriptedRun::emitting(Emission::Unchanged)
            .failing_check(jointly_in_the_first_then_twice_in_the_second)
            .failing_check(thrice_alone_in_the_first);
        let titles = NeedlessTypeFixture::expand(TWICE_AND_THRICE)
            .warning_titles_in_two_programs(TWICE, run);
        assert!(titles.is_empty(), "{titles:?}");
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

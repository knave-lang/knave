use super::declared_functions::DeclaredFunctions;
use super::module_alone_search::ModuleAloneSearch;
use super::needless_type_search::NeedlessTypeSearch;
use super::needless_type_trial::{ByValueRewrites, NeedlessTypeTrial};
use super::needless_type_warnings::NeedlessTypeWarnings;
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

impl<'r, Run: ProgramRun> NeedlessTypeLint<'r, Run> {
    pub fn new(run: &'r mut Run) -> Self {
        Self { run }
    }

    pub fn warnings(
        &mut self,
        level: Option<LintLevel>,
        program: &mut Program,
        declared: TypedProgram,
    ) -> Diagnostics {
        if level == Some(LintLevel::Allow) {
            return Diagnostics::default();
        }
        let candidates = DeclaredFunctions::new(program).candidates();
        let tried = candidates
            .iter()
            .filter(|declaration| declaration.needs_a_trial())
            .map(|declaration| declaration.function)
            .collect();
        let removal = self.removal(program, declared, tried);
        let by_value = removal.kept.unwrap_or_default();
        NeedlessTypeWarnings::new(program).of(&candidates, &removal.removable, &by_value)
    }

    fn removal(
        &mut self,
        program: &mut Program,
        mut declared: TypedProgram,
        mut tried: Vec<BindingId>,
    ) -> Removal<ByValueRewrites> {
        loop {
            let (removal, typed) = self.program_search(program, declared, &tried);
            let accepted = self.accepted_by_modules(program, &removal.removable);
            if accepted.len() == removal.removable.len() {
                return removal;
            }
            declared = typed;
            tried = accepted;
        }
    }

    fn program_search(
        &mut self,
        program: &mut Program,
        declared: TypedProgram,
        tried: &[BindingId],
    ) -> (Removal<ByValueRewrites>, TypedProgram) {
        if tried.is_empty() {
            return (Removal::default(), declared);
        }
        let mut trial = NeedlessTypeTrial::new(&mut *self.run, program, declared);
        let removal = NeedlessTypeSearch::new(&mut trial).jointly_removable(tried);
        (removal, trial.into_declared())
    }

    fn accepted_by_modules(
        &mut self,
        program: &Program,
        removable: &[BindingId],
    ) -> Vec<BindingId> {
        let declared = DeclaredFunctions::new(program);
        let mut refused = Vec::new();
        for library in declared.library_declarations(removable) {
            let accepted = ModuleAloneSearch::new(&mut *self.run).removable_names(&library);
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

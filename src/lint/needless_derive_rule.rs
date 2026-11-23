use super::copy_types::CopyTypes;
use super::declared_derives::DeclaredDerives;
use super::derive_declaration::DeriveDeclaration;
use super::linted_program::LintedProgram;
use super::module_checks::ModuleChecks;
use super::needless_derive_trial::NeedlessDeriveTrial;
use super::problems::LintProblems;
use super::program_run::ProgramRun;
use super::project_library::ProjectLibrary;
use super::removal::Removal;
use super::trial_candidate::TrialCandidate;
use super::trial_rule::TrialRule;
use super::written_program::WrittenProgram;
use crate::core::{BindingId, CoreModule};
use crate::diagnostic::Diagnostics;
use crate::expand::Program;
use crate::types::{InferableDerives, TypedProgram};

pub struct NeedlessDeriveRule {
    library: ProjectLibrary,
}

impl NeedlessDeriveRule {
    pub fn new(library: ProjectLibrary) -> Self {
        Self { library }
    }

    fn forms_to_try(program: &Program, declared: &TypedProgram) -> Vec<DeriveDeclaration> {
        let written = DeclaredDerives::new(program).written();
        if written.is_empty() {
            return written;
        }
        let inferable = InferableDerives::of(
            &program.modules,
            declared.declarations.traits().structural(),
        );
        DeclaredDerives::inferred_among(written, &inferable)
    }

    fn keeps_copies(&self, linted: &LintedProgram<'_>) -> bool {
        linted.holds_unlinted_example || self.library == ProjectLibrary::Published
    }

    fn possibly_copy(declared: &TypedProgram, removal: &Removal<Vec<BindingId>>) -> Vec<BindingId> {
        let mut copies = CopyTypes::new(declared).possibly_copy(&removal.removable);
        copies.extend(removal.kept.iter().flatten().copied());
        copies
    }
}

impl TrialRule for NeedlessDeriveRule {
    type Declaration = DeriveDeclaration;
    type Kept = Vec<BindingId>;
    type Trial<'r, Run: ProgramRun + 'r> = NeedlessDeriveTrial<'r, Run>;

    fn candidates(&self, linted: &LintedProgram<'_>) -> Vec<DeriveDeclaration> {
        Self::forms_to_try(linted.program, linted.declared)
    }

    fn candidate<'d>(&self, declaration: &'d DeriveDeclaration) -> TrialCandidate<'d> {
        TrialCandidate {
            binding: declaration.target,
            module: &declaration.module,
            span: declaration.span,
            tried: true,
        }
    }

    fn declared_in(&self, program: &Program, module: &CoreModule) -> Vec<BindingId> {
        DeclaredDerives::new(program)
            .declared_in(module)
            .iter()
            .map(|derive| derive.target)
            .collect()
    }

    fn trial<'r, Run: ProgramRun>(
        &self,
        run: &'r mut Run,
        program: &'r mut Program,
        declared: &'r TypedProgram,
    ) -> NeedlessDeriveTrial<'r, Run> {
        NeedlessDeriveTrial::new(run, program, declared)
    }

    fn module_checks(
        &self,
        linted: &LintedProgram<'_>,
        removal: &Removal<Vec<BindingId>>,
    ) -> ModuleChecks {
        let copies = Self::possibly_copy(linted.declared, removal);
        if self.keeps_copies(linted) {
            ModuleChecks {
                refused: copies,
                checked_alone: Vec::new(),
            }
        } else {
            ModuleChecks {
                refused: Vec::new(),
                checked_alone: copies,
            }
        }
    }

    fn warnings(
        &self,
        program: &Program,
        declarations: &[DeriveDeclaration],
        warned: Removal<Vec<BindingId>>,
    ) -> Diagnostics {
        let written = WrittenProgram::new(program);
        let mut warnings = Diagnostics::default();
        for derive in declarations
            .iter()
            .filter(|derive| warned.removable.contains(&derive.target))
        {
            warnings.push(LintProblems::needless_derive(
                &written.name(derive.target),
                derive.span,
            ));
        }
        warnings
    }
}

#[cfg(test)]
mod tests {
    use crate::lint::needless_type_fixture::{
        DeriveTrials, Emission, NeedlessTypeFixture, QuestionExpansion, ScriptedRun,
    };

    const COMPARED: &str = "(defstruct point :x int :y int)

(derive point clone partial-eq)

(defn main () (println (= (->point :x 1 :y 2) (->point :x 1 :y 2))))
";

    const DEBUGGED: &str = "(defstruct point :x int :y int)

(derive point clone debug)

(defn main () (println (:x (->point :x 1 :y 2))))
";

    fn trials_of(fixture: NeedlessTypeFixture) -> DeriveTrials {
        fixture.derive_trials(ScriptedRun::emitting(Emission::Unchanged))
    }

    #[test]
    fn a_form_that_inference_gives_is_tried_and_warns() {
        let trials = trials_of(NeedlessTypeFixture::expand(COMPARED));
        assert_eq!((trials.warnings, trials.checks), (1, 1));
    }

    #[test]
    fn a_question_that_fails_to_expand_keeps_every_copy_possible_form() {
        let run =
            ScriptedRun::emitting(Emission::Unchanged).expanding_question(QuestionExpansion::Fails);
        let trials = NeedlessTypeFixture::expand(COMPARED).derive_trials(run);
        assert_eq!(trials.warnings, 0);
    }

    #[test]
    fn a_form_that_lists_a_trait_inference_never_gives_runs_no_trial() {
        let trials = trials_of(NeedlessTypeFixture::expand(DEBUGGED));
        assert_eq!((trials.warnings, trials.checks), (0, 0));
    }

    #[test]
    fn a_library_type_that_is_copy_is_checked_in_its_module_alone() {
        let trials = trials_of(NeedlessTypeFixture::expand_derive_fixture(
            "needless_derive_lib",
        ));
        assert_eq!(trials.alone_loads, 1);
    }

    #[test]
    fn a_library_type_that_is_never_copy_is_not_checked_alone() {
        let trials = trials_of(NeedlessTypeFixture::expand_derive_fixture(
            "needless_derive_text",
        ));
        assert_eq!((trials.warnings, trials.alone_loads), (1, 0));
    }
}

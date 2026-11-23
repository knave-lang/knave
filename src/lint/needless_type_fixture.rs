use std::cell::Cell;
use std::path::{Path, PathBuf};

use super::example_question::ExampleQuestion;
use super::example_runs::ExampleRuns;
use super::linted_program::LintedProgram;
use super::needless_derive_rule::NeedlessDeriveRule;
use super::needless_type_rule::NeedlessTypeRule;
use super::problems::LintProblems;
use super::program_position::ProgramPosition;
use super::program_run::ProgramRun;
use super::project_library::ProjectLibrary;
use super::rule_failure::RuleFailure;
use super::trial_levels::TrialLevels;
use super::trial_lint::TrialLint;
use super::trial_lints::TrialLints;
use crate::check::ProgramChecker;
use crate::core::ModulePath;
use crate::diagnostic::Diagnostics;
use crate::expand::{Expander, Program};
use crate::modules::ModuleSources;
use crate::ownership::EnforcedOwnership;
use crate::project::LintLevel;
use crate::source::{SourceId, SourceMap, Span};
use crate::types::TypedProgram;

#[derive(Clone, Copy)]
pub enum Emission {
    Unchanged,
    DifferentEachTime,
}

pub struct ScriptedRun {
    emission: Emission,
    failing_checks: Vec<usize>,
    checks: usize,
    alone_loads: usize,
    emissions: Cell<usize>,
    question: QuestionExpansion,
    sources: SourceMap,
    question_source: SourceId,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum QuestionExpansion {
    Expands,
    Fails,
}

impl ScriptedRun {
    const QUESTION: &str = "notes-hold-example?";

    pub fn emitting(emission: Emission) -> Self {
        let mut sources = SourceMap::default();
        let question_source = sources.add("question", String::new());
        Self {
            emission,
            failing_checks: Vec::new(),
            checks: 0,
            alone_loads: 0,
            emissions: Cell::new(0),
            question: QuestionExpansion::Expands,
            sources,
            question_source,
        }
    }

    pub fn expanding_question(self, question: QuestionExpansion) -> Self {
        Self { question, ..self }
    }

    pub fn failing_check(mut self, trial_check: usize) -> Self {
        self.failing_checks.push(trial_check);
        self
    }
}

impl ProgramRun for ScriptedRun {
    fn check(&mut self, program: &Program) -> Result<TypedProgram, Diagnostics> {
        self.checks += 1;
        if self.failing_checks.contains(&self.checks) {
            return Err(Diagnostics::default());
        }
        ProgramChecker::check::<EnforcedOwnership>(program)
    }

    fn emit(&self, _typed: &TypedProgram) -> Option<String> {
        self.emissions.set(self.emissions.get() + 1);
        Some(match self.emission {
            Emission::Unchanged => String::new(),
            Emission::DifferentEachTime => self.emissions.get().to_string(),
        })
    }

    fn module_alone(&mut self, _beside: ProgramPosition, _module: &ModulePath) -> Option<Program> {
        self.alone_loads += 1;
        None
    }

    fn source_path(&self, _source: SourceId) -> Option<PathBuf> {
        None
    }

    fn example_question(&mut self) -> Result<ExampleQuestion, RuleFailure> {
        match self.question {
            QuestionExpansion::Expands => ExampleQuestion::expand_from_sources(),
            QuestionExpansion::Fails => Err(RuleFailure::of(
                LintProblems::missing_export(
                    ModuleSources::EXAMPLES,
                    Self::QUESTION,
                    Span::new(self.question_source, 0..0),
                ),
                &self.sources,
            )),
        }
    }

    fn sources(&self) -> &SourceMap {
        &self.sources
    }
}

pub struct NeedlessTypeFixture {
    program: Program,
}

impl NeedlessTypeFixture {
    const MODULE: &str = "fixture";

    pub fn expand(text: &str) -> Self {
        Self::expand_in(PathBuf::from("."), Self::MODULE, text)
    }

    const FIXTURES: &str = "tests/fixtures/lint/needless_type";
    const LINT_FIXTURES: &str = "tests/fixtures/lint";

    pub fn expand_lint_fixture(name: &str) -> Self {
        Self::expand_fixture_in(Self::FIXTURES, name)
    }

    pub fn expand_derive_fixture(name: &str) -> Self {
        Self::expand_fixture_in(Self::LINT_FIXTURES, name)
    }

    fn expand_fixture_in(fixtures: &str, name: &str) -> Self {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join(fixtures);
        let text = std::fs::read_to_string(directory.join(format!("{name}.knv")))
            .expect("the fixture is readable");
        Self::expand_in(directory, name, &text)
    }

    fn expand_in(directory: PathBuf, module: &str, text: &str) -> Self {
        let mut sources = SourceMap::default();
        let entry = sources.add(module, text);
        let program = Expander::new(
            &mut sources,
            ModuleSources::new(directory.clone(), directory),
        )
        .expand(ModulePath::new(module), entry)
        .expect("the source expands");
        Self { program }
    }

    pub fn warning_titles(self, run: ScriptedRun) -> Vec<String> {
        self.warning_titles_at(Some(LintLevel::Warn), run).0
    }

    pub fn warning_titles_at(
        mut self,
        written: Option<LintLevel>,
        mut run: ScriptedRun,
    ) -> (Vec<String>, usize) {
        let declared =
            ProgramChecker::check::<EnforcedOwnership>(&self.program).expect("the source checks");
        let mut programs = [LintedProgram {
            program: &mut self.program,
            declared: &declared,
            examples: ExampleRuns::NoModule,
            holds_unlinted_example: false,
        }];
        let mut lint = TrialLint::new(&mut run, &NeedlessTypeRule);
        let candidates = lint.candidates(written, &programs);
        let warnings = lint.outcome(candidates, &mut programs).warnings;
        let titles = warnings
            .into_iter()
            .map(|warning| warning.title().to_string())
            .collect();
        (titles, run.checks)
    }

    pub fn warning_titles_in_two_programs(
        mut self,
        text: &str,
        mut run: ScriptedRun,
    ) -> Vec<String> {
        let mut second = Self::expand(text).program;
        let first_checked =
            ProgramChecker::check::<EnforcedOwnership>(&self.program).expect("the source checks");
        let second_checked =
            ProgramChecker::check::<EnforcedOwnership>(&second).expect("the source checks");
        let mut programs = [
            LintedProgram {
                program: &mut self.program,
                declared: &first_checked,
                examples: ExampleRuns::NoModule,
                holds_unlinted_example: false,
            },
            LintedProgram {
                program: &mut second,
                declared: &second_checked,
                examples: ExampleRuns::NoModule,
                holds_unlinted_example: false,
            },
        ];
        let mut lint = TrialLint::new(&mut run, &NeedlessTypeRule);
        let candidates = lint.candidates(Some(LintLevel::Warn), &programs);
        lint.outcome(candidates, &mut programs)
            .warnings
            .into_iter()
            .map(|warning| warning.title().to_string())
            .collect()
    }
    pub fn derive_trials(mut self, mut run: ScriptedRun) -> DeriveTrials {
        let declared =
            ProgramChecker::check::<EnforcedOwnership>(&self.program).expect("the source checks");
        let mut programs = [LintedProgram {
            program: &mut self.program,
            declared: &declared,
            examples: ExampleRuns::NoModule,
            holds_unlinted_example: false,
        }];
        let levels = TrialLevels {
            needless_type: Some(LintLevel::Allow),
            needless_derive: Some(LintLevel::Warn),
        };
        let rule = NeedlessDeriveRule::new(ProjectLibrary::Unpublished);
        let warnings = TrialLints::new(&mut run, rule).warnings(levels, &mut programs);
        DeriveTrials {
            warnings: warnings.len(),
            checks: run.checks,
            alone_loads: run.alone_loads,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct DeriveTrials {
    pub warnings: usize,
    pub checks: usize,
    pub alone_loads: usize,
}

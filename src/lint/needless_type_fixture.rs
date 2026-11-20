use std::cell::Cell;
use std::path::{Path, PathBuf};

use super::linted_program::LintedProgram;
use super::needless_derive_rule::NeedlessDeriveRule;
use super::needless_type_rule::NeedlessTypeRule;
use super::program_position::ProgramPosition;
use super::program_run::ProgramRun;
use super::project_library::ProjectLibrary;
use super::trial_lint::TrialLint;
use crate::check::ProgramChecker;
use crate::core::ModulePath;
use crate::diagnostic::Diagnostics;
use crate::expand::{Expander, Program};
use crate::modules::ModuleSources;
use crate::ownership::EnforcedOwnership;
use crate::project::LintLevel;
use crate::source::{SourceId, SourceMap};
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
}

impl ScriptedRun {
    pub fn emitting(emission: Emission) -> Self {
        Self {
            emission,
            failing_checks: Vec::new(),
            checks: 0,
            alone_loads: 0,
            emissions: Cell::new(0),
        }
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
            holds_unlinted_example: false,
        }];
        let warnings = TrialLint::new(&mut run, &NeedlessTypeRule)
            .outcome(written, &mut programs)
            .warnings;
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
                holds_unlinted_example: false,
            },
            LintedProgram {
                program: &mut second,
                declared: &second_checked,
                holds_unlinted_example: false,
            },
        ];
        TrialLint::new(&mut run, &NeedlessTypeRule)
            .outcome(Some(LintLevel::Warn), &mut programs)
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
            holds_unlinted_example: false,
        }];
        let rule = NeedlessDeriveRule::new(ProjectLibrary::Unpublished);
        let warnings = TrialLint::new(&mut run, &rule)
            .outcome(Some(LintLevel::Warn), &mut programs)
            .warnings;
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

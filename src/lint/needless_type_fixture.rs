use std::cell::Cell;
use std::path::{Path, PathBuf};

use super::needless_type_lint::NeedlessTypeLint;
use super::program_run::ProgramRun;
use crate::check::ProgramChecker;
use crate::core::ModulePath;
use crate::diagnostic::Diagnostics;
use crate::expand::{Expander, Program};
use crate::modules::ModuleSources;
use crate::ownership::EnforcedOwnership;
use crate::project::LintLevel;
use crate::source::SourceMap;
use crate::types::TypedProgram;

#[derive(Clone, Copy)]
pub enum Emission {
    Unchanged,
    DifferentEachTime,
}

pub struct ScriptedRun {
    emission: Emission,
    failing_check: Option<usize>,
    checks: usize,
    emissions: Cell<usize>,
}

impl ScriptedRun {
    pub fn emitting(emission: Emission) -> Self {
        Self {
            emission,
            failing_check: None,
            checks: 0,
            emissions: Cell::new(0),
        }
    }

    pub fn failing_check(self, trial_check: usize) -> Self {
        Self {
            failing_check: Some(trial_check),
            ..self
        }
    }
}

impl ProgramRun for ScriptedRun {
    fn check(&mut self, program: &Program) -> Result<TypedProgram, Diagnostics> {
        self.checks += 1;
        match self.failing_check {
            Some(failing) if failing == self.checks => Err(Diagnostics::default()),
            _ => ProgramChecker::check::<EnforcedOwnership>(program),
        }
    }

    fn emit(&self, _typed: &TypedProgram) -> Option<String> {
        self.emissions.set(self.emissions.get() + 1);
        Some(match self.emission {
            Emission::Unchanged => String::new(),
            Emission::DifferentEachTime => self.emissions.get().to_string(),
        })
    }

    fn module_alone(&mut self, _module: &ModulePath) -> Option<Program> {
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

    pub fn expand_lint_fixture(name: &str) -> Self {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join(Self::FIXTURES);
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
        let warnings =
            NeedlessTypeLint::new(&mut run).warnings(written, &mut self.program, declared);
        let titles = warnings
            .into_iter()
            .map(|warning| warning.title().to_string())
            .collect();
        (titles, run.checks)
    }
}

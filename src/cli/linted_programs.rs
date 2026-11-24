use super::embedded_prelude::EmbeddedPrelude;
use super::exit_status::ExitStatus;
use super::loaded_programs::LoadedPrograms;
use super::program_file::ProgramFile;
use super::project_program::ProjectProgram;
use super::terminal::Terminal;
use super::trial_runs::TrialRuns;
use crate::diagnostic::{DiagnosticCode, Diagnostics};
use crate::expand::Program;
use crate::lint::{
    AgreedWarnings, LintedProgram, Linter, NeedlessTypeLint, ProgramWarnings, RuleFailure,
    RuleProgram,
};
use crate::types::TypedProgram;

pub struct LintedPrograms {
    files: Vec<ProgramFile>,
    checked: Vec<CheckedProgram>,
    failure: Option<ExitStatus>,
}

struct CheckedProgram {
    program: Program,
    typed: TypedProgram,
    warnings: Diagnostics,
}

impl LintedPrograms {
    pub fn checked(terminal: &mut Terminal, loaded: LoadedPrograms) -> Self {
        let mut linted = Self {
            files: Vec::new(),
            checked: Vec::new(),
            failure: None,
        };
        for result in loaded.into_results() {
            if let Err(status) = result.and_then(|program| linted.add_checked(terminal, program)) {
                linted.failure.get_or_insert(status);
            }
        }
        linted
    }

    fn add_checked(
        &mut self,
        terminal: &mut Terminal,
        ProjectProgram { file, program }: ProjectProgram,
    ) -> Result<(), ExitStatus> {
        let mut typed = terminal.checking().check_keeping_warnings(&program)?;
        let warnings = std::mem::take(&mut typed.warnings);
        self.files.push(file);
        self.checked.push(CheckedProgram {
            program,
            typed,
            warnings,
        });
        Ok(())
    }

    pub fn failure(&self) -> Option<ExitStatus> {
        self.failure
    }

    pub fn warnings(mut self, terminal: &mut Terminal) -> Result<Diagnostics, ExitStatus> {
        let lints = self.rule_lints(terminal)?;
        let needless_types = match self.failure {
            Some(_) => Diagnostics::default(),
            None => self.needless_types(terminal),
        };
        let programs = self
            .checked
            .iter()
            .zip(lints)
            .map(|(checked, lints)| {
                let mut warnings = checked.warnings.clone();
                warnings.extend(lints);
                ProgramWarnings {
                    program: &checked.program,
                    warnings,
                }
            })
            .collect();
        let mut warnings = AgreedWarnings::new(terminal.source_report().sources()).of(programs);
        warnings.extend(needless_types);
        Ok(warnings)
    }

    fn rule_lints(&self, terminal: &Terminal) -> Result<Vec<Diagnostics>, ExitStatus> {
        let report = terminal.source_report();
        let failed = |failure: RuleFailure| {
            report.note(failure.report());
            ExitStatus::ProgramErrors
        };
        let mut rules = RuleProgram::expand_on(EmbeddedPrelude::state()).map_err(failed)?;
        self.checked
            .iter()
            .map(|checked| {
                Linter::new(&checked.program, report.sources())
                    .lint(&mut rules)
                    .map_err(failed)
            })
            .collect()
    }

    fn needless_types(&mut self, terminal: &mut Terminal) -> Diagnostics {
        let level = terminal
            .source_report()
            .lints()
            .level_of(&DiagnosticCode::NEEDLESS_TYPE);
        let mut linted: Vec<LintedProgram<'_>> = self
            .checked
            .iter_mut()
            .map(|checked| LintedProgram {
                program: &mut checked.program,
                declared: &checked.typed,
            })
            .collect();
        NeedlessTypeLint::new(&mut TrialRuns::new(terminal, &self.files))
            .warnings(level, &mut linted)
    }
}

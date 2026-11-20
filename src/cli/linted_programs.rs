use super::embedded_prelude::EmbeddedPrelude;
use super::example_runs::ExampleRuns;
use super::exit_status::ExitStatus;
use super::loaded_programs::LoadedPrograms;
use super::program_file::ProgramFile;
use super::project_program::ProjectProgram;
use super::terminal::Terminal;
use super::trial_runs::TrialRuns;
use crate::diagnostic::{DiagnosticCode, Diagnostics};
use crate::expand::Program;
use crate::lint::{
    AgreedWarnings, LintedProgram, Linter, NeedlessDeriveRule, ProgramWarnings, ProjectLibrary,
    RuleFailure, RuleProgram, TrialLevels, TrialLints,
};
use crate::types::TypedProgram;

pub struct LintedPrograms {
    files: Vec<ProgramFile>,
    checked: Vec<CheckedProgram>,
    failure: Option<ExitStatus>,
    library: ProjectLibrary,
    examples: ExampleRuns,
}

struct CheckedProgram {
    program: Program,
    typed: TypedProgram,
    warnings: Diagnostics,
}

impl LintedPrograms {
    pub fn checked(terminal: &mut Terminal, loaded: LoadedPrograms) -> Self {
        let library = loaded.library();
        let examples = loaded.examples();
        let (programs, mut failure) = loaded.into_parts();
        let mut files = Vec::new();
        let mut checked = Vec::new();
        for ProjectProgram { file, program } in programs {
            match terminal.checking().check_keeping_warnings(&program) {
                Ok(mut typed) => {
                    let warnings = std::mem::take(&mut typed.warnings);
                    files.push(file);
                    checked.push(CheckedProgram {
                        program,
                        typed,
                        warnings,
                    });
                }
                Err(status) => failure = Some(status),
            }
        }
        Self {
            files,
            checked,
            failure,
            library,
            examples,
        }
    }

    pub fn failure(&self) -> Option<ExitStatus> {
        self.failure
    }

    pub fn warnings(mut self, terminal: &mut Terminal) -> Result<Diagnostics, ExitStatus> {
        let mut rules = RuleProgram::expand_on(EmbeddedPrelude::state())
            .map_err(|failure| Self::failed(terminal, failure))?;
        let lints = self.rule_lints(terminal, &mut rules)?;
        let trial_lints = match self.failure {
            Some(_) => Diagnostics::default(),
            None => {
                let unlinted = self.unlinted_examples(terminal, &mut rules)?;
                self.trial_lints(terminal, &unlinted)
            }
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
        warnings.extend(trial_lints);
        Ok(warnings)
    }

    fn failed(terminal: &Terminal, failure: RuleFailure) -> ExitStatus {
        terminal.source_report().note(failure.report());
        ExitStatus::ProgramErrors
    }

    fn rule_lints(
        &self,
        terminal: &Terminal,
        rules: &mut RuleProgram,
    ) -> Result<Vec<Diagnostics>, ExitStatus> {
        let sources = terminal.source_report().sources();
        self.checked
            .iter()
            .map(|checked| {
                Linter::new(&checked.program, sources)
                    .lint(rules)
                    .map_err(|failure| Self::failed(terminal, failure))
            })
            .collect()
    }

    fn unlinted_examples(
        &self,
        terminal: &Terminal,
        rules: &mut RuleProgram,
    ) -> Result<Vec<bool>, ExitStatus> {
        let sources = terminal.source_report().sources();
        self.checked
            .iter()
            .map(|checked| match self.examples {
                ExampleRuns::Linted => Ok(false),
                ExampleRuns::Unlinted => Linter::new(&checked.program, sources)
                    .holds_example(rules)
                    .map_err(|failure| Self::failed(terminal, failure)),
            })
            .collect()
    }

    fn trial_lints(&mut self, terminal: &mut Terminal, unlinted: &[bool]) -> Diagnostics {
        let lints = terminal.source_report().lints();
        let levels = TrialLevels {
            needless_type: lints.level_of(&DiagnosticCode::NEEDLESS_TYPE),
            needless_derive: lints.level_of(&DiagnosticCode::NEEDLESS_DERIVE),
        };
        let mut linted: Vec<LintedProgram<'_>> = self
            .checked
            .iter_mut()
            .zip(unlinted)
            .map(|(checked, unlinted)| LintedProgram {
                program: &mut checked.program,
                declared: &checked.typed,
                holds_unlinted_example: *unlinted,
            })
            .collect();
        let mut runs = TrialRuns::new(terminal, &self.files);
        TrialLints::new(&mut runs, NeedlessDeriveRule::new(self.library))
            .warnings(levels, &mut linted)
    }
}

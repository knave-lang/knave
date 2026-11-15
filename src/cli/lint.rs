use super::embedded_prelude::EmbeddedPrelude;
use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::program_file::ProgramFile;
use super::project_inspection::{ProjectInspection, WarningPrinter};
use super::terminal::Terminal;
use crate::lint::{Linter, RuleFailure, RuleProgram};

pub struct LintCommand {
    entry: EntryArgument,
}

impl LintCommand {
    pub fn new(entry: EntryArgument) -> Self {
        Self { entry }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        let inspected = ProjectInspection::new(self.entry).run(
            terminal,
            &|report, warnings| report.print_lints(warnings),
            Self::lint,
        );
        ExitStatus::of(inspected)
    }

    fn lint(
        terminal: &mut Terminal,
        file: &ProgramFile,
        print_warnings: &WarningPrinter,
    ) -> Result<(), ExitStatus> {
        let entry = terminal.sources().load(file.path())?;
        let program = terminal.expansion().expand(file, entry)?;
        let mut warnings = terminal
            .checking()
            .check_keeping_warnings(&program)?
            .warnings;
        let report = terminal.source_report();
        let linter = Linter::new(&program, report.sources());
        let lints = Self::rules()
            .and_then(|mut rules| linter.lint(&mut rules))
            .map_err(|failure| {
                report.note(failure.report());
                ExitStatus::ProgramErrors
            })?;
        warnings.extend(lints);
        print_warnings(&report, &warnings).passed()
    }

    fn rules() -> Result<RuleProgram, RuleFailure> {
        RuleProgram::expand_on(EmbeddedPrelude::state())
    }
}

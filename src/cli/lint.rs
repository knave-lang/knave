use super::embedded_prelude::EmbeddedPrelude;
use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::program_file::ProgramFile;
use super::project_inspection::{ProjectInspection, WarningPrinter};
use super::terminal::Terminal;
use super::trial_runs::TrialRuns;
use crate::diagnostic::DiagnosticCode;
use crate::lint::{Linter, NeedlessTypeLint, RuleFailure, RuleProgram};

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
            &|report, warnings| report.lints().print(warnings),
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
        let mut program = terminal.expansion().expand(file, entry)?;
        let mut checked = terminal.checking().check_keeping_warnings(&program)?;
        let mut warnings = std::mem::take(&mut checked.warnings);
        let level = terminal
            .source_report()
            .lints()
            .level_of(&DiagnosticCode::NEEDLESS_TYPE);
        let needless_types = NeedlessTypeLint::new(&mut TrialRuns::new(terminal, file)).warnings(
            level,
            &mut program,
            checked,
        );
        let report = terminal.source_report();
        let linter = Linter::new(&program, report.sources());
        let lints = Self::rules()
            .and_then(|mut rules| linter.lint(&mut rules))
            .map_err(|failure| {
                report.note(failure.report());
                ExitStatus::ProgramErrors
            })?;
        warnings.extend(lints);
        warnings.extend(needless_types);
        print_warnings(&report, &warnings).passed()
    }

    fn rules() -> Result<RuleProgram, RuleFailure> {
        RuleProgram::expand_on(EmbeddedPrelude::state())
    }
}

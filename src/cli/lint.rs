use super::embedded_prelude::EmbeddedPrelude;
use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::program_file::ProgramFile;
use super::project_inspection::ProjectInspection;
use super::terminal::Terminal;
use crate::diagnostic::{CodeStage, Diagnostics};
use crate::lint::{Linter, RuleFailure, RuleProgram};

pub struct LintCommand {
    entry: EntryArgument,
}

impl LintCommand {
    pub fn new(entry: EntryArgument) -> Self {
        Self { entry }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        let inspected = ProjectInspection::new(self.entry)
            .run(terminal, Self::lint)
            .and_then(Self::passed);
        ExitStatus::of(inspected)
    }

    fn lint(terminal: &mut Terminal, file: &ProgramFile) -> Result<Diagnostics, ExitStatus> {
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
        report.print(&warnings);
        Ok(warnings)
    }

    fn passed(printed: Diagnostics) -> Result<Diagnostics, ExitStatus> {
        if Self::holds_a_lint(&printed) {
            return Err(ExitStatus::ProgramErrors);
        }
        Ok(printed)
    }

    fn holds_a_lint(warnings: &Diagnostics) -> bool {
        warnings.in_source_order().into_iter().any(|warning| {
            warning
                .code()
                .is_some_and(|code| CodeStage::LINTS.contains(code))
        })
    }

    fn rules() -> Result<RuleProgram, RuleFailure> {
        RuleProgram::expand_on(EmbeddedPrelude::state())
    }
}

#[cfg(test)]
mod tests {
    use super::LintCommand;
    use crate::diagnostic::{Diagnostic, DiagnosticCode, Diagnostics, Label};
    use crate::source::{SourceId, Span};

    struct WarningsFixture;

    impl WarningsFixture {
        fn of(code: DiagnosticCode) -> Diagnostics {
            let span = Span::new(SourceId::new(0), 0..1);
            let mut warnings = Diagnostics::default();
            warnings.push(Diagnostic::warning(
                code,
                "a warning",
                Label::new(span, "here"),
            ));
            warnings
        }
    }

    #[test]
    fn a_skipped_rule_is_a_lint() {
        let warnings = WarningsFixture::of(DiagnosticCode::RULE_SKIPPED);
        assert!(LintCommand::holds_a_lint(&warnings));
    }

    #[test]
    fn a_second_trait_path_is_no_lint() {
        let warnings = WarningsFixture::of(DiagnosticCode::SECOND_TRAIT_PATH);
        assert!(!LintCommand::holds_a_lint(&warnings));
    }

    #[test]
    fn no_warning_holds_no_lint() {
        assert!(!LintCommand::holds_a_lint(&Diagnostics::default()));
    }
}

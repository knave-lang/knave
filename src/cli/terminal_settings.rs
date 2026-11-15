use std::rc::Rc;

use super::diagnostic_output::DiagnosticOutput;
use super::interruption::Interruption;
use super::lint_levels::LintLevels;
use super::source_report::SourceReport;
use crate::source::SourceMap;

pub struct TerminalSettings {
    diagnostics: Rc<dyn DiagnosticOutput>,
    interruption: Interruption,
    lint_levels: LintLevels,
}

impl TerminalSettings {
    pub fn reporting_to(diagnostics: Rc<dyn DiagnosticOutput>) -> Self {
        Self {
            diagnostics,
            interruption: Interruption::default(),
            lint_levels: LintLevels::default(),
        }
    }

    pub fn stopping_on(self, interruption: Interruption) -> Self {
        Self {
            interruption,
            ..self
        }
    }

    pub fn rerouted_to(&self, diagnostics: Rc<dyn DiagnosticOutput>) -> Self {
        Self {
            diagnostics,
            interruption: self.interruption.clone(),
            lint_levels: self.lint_levels.clone(),
        }
    }

    pub fn leveling_lints_by(&self, levels: LintLevels) -> Self {
        Self {
            diagnostics: Rc::clone(&self.diagnostics),
            interruption: self.interruption.clone(),
            lint_levels: levels,
        }
    }

    pub fn source_report<'a>(&'a self, sources: &'a SourceMap) -> SourceReport<'a> {
        SourceReport::new(sources, self.diagnostics.as_ref(), &self.lint_levels)
    }

    pub fn diagnostics(&self) -> &Rc<dyn DiagnosticOutput> {
        &self.diagnostics
    }

    pub fn interruption(&self) -> &Interruption {
        &self.interruption
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::TerminalSettings;
    use crate::cli::diagnostic_output::DiagnosticOutput;
    use crate::cli::interruption::Interruption;
    use crate::cli::recorded_output::RecordedOutput;

    struct SettingsFixture {
        output: Rc<dyn DiagnosticOutput>,
    }

    impl SettingsFixture {
        fn new() -> Self {
            Self {
                output: Rc::new(RecordedOutput::default()),
            }
        }

        fn reporting(&self) -> TerminalSettings {
            TerminalSettings::reporting_to(Rc::clone(&self.output))
        }

        fn reports_to_its_output(&self, settings: &TerminalSettings) -> bool {
            Rc::ptr_eq(settings.diagnostics(), &self.output)
        }
    }

    #[test]
    fn reports_to_the_output_it_is_given() {
        let fixture = SettingsFixture::new();
        assert!(fixture.reports_to_its_output(&fixture.reporting()));
    }

    #[test]
    fn stops_on_nothing_until_told() {
        let fixture = SettingsFixture::new();
        assert!(!fixture.reporting().interruption().is_raised());
    }

    #[test]
    fn stops_on_the_interruption_it_is_given() {
        let fixture = SettingsFixture::new();
        let settings = fixture.reporting().stopping_on(Interruption::raised());
        assert!(settings.interruption().is_raised());
    }

    #[test]
    fn keeps_its_output_when_told_to_stop() {
        let fixture = SettingsFixture::new();
        let settings = fixture.reporting().stopping_on(Interruption::raised());
        assert!(fixture.reports_to_its_output(&settings));
    }

    #[test]
    fn keeps_its_interruption_when_rerouted() {
        let fixture = SettingsFixture::new();
        let stopping = fixture.reporting().stopping_on(Interruption::raised());
        let rerouted = stopping.rerouted_to(Rc::new(RecordedOutput::default()));
        assert!(rerouted.interruption().is_raised());
    }

    #[test]
    fn reports_to_the_output_it_is_rerouted_to() {
        let fixture = SettingsFixture::new();
        let rerouted = TerminalSettings::reporting_to(Rc::new(RecordedOutput::default()))
            .rerouted_to(Rc::clone(&fixture.output));
        assert!(fixture.reports_to_its_output(&rerouted));
    }
}

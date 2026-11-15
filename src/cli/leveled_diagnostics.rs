use std::borrow::Cow;

use super::lint_levels::LintLevels;
use super::lint_setting::{LintBehavior, LintSetting};
use crate::diagnostic::{Diagnostic, Diagnostics, Severity};

pub struct LeveledDiagnostics<'d> {
    input: &'d Diagnostics,
    levels: &'d LintLevels,
}

impl<'d> LeveledDiagnostics<'d> {
    pub fn new(input: &'d Diagnostics, levels: &'d LintLevels) -> Self {
        Self { input, levels }
    }

    pub fn shown(&self) -> Cow<'d, Diagnostics> {
        if self.levels.is_empty() {
            return Cow::Borrowed(self.input);
        }
        Cow::Owned(
            self.input
                .into_iter()
                .filter_map(|diagnostic| self.leveled(diagnostic))
                .collect(),
        )
    }

    pub fn denied(&self) -> Option<Diagnostics> {
        if self.levels.is_empty() {
            return None;
        }
        let denied: Diagnostics = self
            .input
            .into_iter()
            .filter(|diagnostic| {
                self.behavior_of(diagnostic)
                    .is_some_and(|behavior| behavior.denied)
            })
            .cloned()
            .collect();
        (!denied.is_empty()).then_some(denied)
    }

    pub fn fails_lint(&self) -> bool {
        self.input.into_iter().any(|diagnostic| {
            self.behavior_of(diagnostic)
                .is_some_and(|behavior| behavior.fails_lint)
        })
    }

    fn leveled(&self, diagnostic: &Diagnostic) -> Option<Diagnostic> {
        match self.behavior_of(diagnostic) {
            Some(LintBehavior { shown: false, .. }) => None,
            Some(LintBehavior { denied: true, .. }) => Some(Self::raised(diagnostic)),
            _ => Some(diagnostic.clone()),
        }
    }

    fn behavior_of(&self, diagnostic: &Diagnostic) -> Option<LintBehavior> {
        self.levels
            .setting_of(diagnostic)
            .map(LintSetting::behavior)
    }

    fn raised(diagnostic: &Diagnostic) -> Diagnostic {
        diagnostic.clone().into_severity(Severity::Error)
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::LeveledDiagnostics;
    use crate::cli::lint_levels::LintLevels;
    use crate::diagnostic::{Diagnostic, DiagnosticCode, Diagnostics, Label, Severity};
    use crate::project::LintLevel;
    use crate::source::{SourceId, Span};

    struct LevelsFixture;

    impl LevelsFixture {
        fn levels(level: LintLevel) -> LintLevels {
            [(DiagnosticCode::UNUSED_BINDING, level)]
                .into_iter()
                .collect()
        }

        fn warning(code: DiagnosticCode, start: usize) -> Diagnostic {
            let span = Span::new(SourceId::new(0), start..start + 1);
            Diagnostic::warning(code, "a fake warning", Label::new(span, "here"))
        }

        fn warnings(code: DiagnosticCode) -> Diagnostics {
            Diagnostics::from(Self::warning(code, 0))
        }

        fn shown(levels: &LintLevels, warnings: &Diagnostics) -> (Vec<Severity>, bool) {
            let leveled = LeveledDiagnostics::new(warnings, levels);
            let severities = leveled
                .shown()
                .in_source_order()
                .into_iter()
                .map(Diagnostic::severity)
                .collect();
            (severities, leveled.fails_lint())
        }

        fn unused_binding_under(levels: &LintLevels) -> (Vec<Severity>, bool) {
            Self::shown(levels, &Self::warnings(DiagnosticCode::UNUSED_BINDING))
        }
    }

    #[test]
    fn a_lint_with_no_level_prints_as_a_warning_and_fails_knave_lint() {
        let shown = LevelsFixture::unused_binding_under(&LintLevels::default());
        assert_eq!(shown, (vec![Severity::Warning], true));
    }

    #[test]
    fn a_lint_with_no_level_beside_another_level_fails_knave_lint() {
        let levels = [(DiagnosticCode::UNREACHABLE_ARM, LintLevel::Allow)]
            .into_iter()
            .collect();
        let shown = LevelsFixture::unused_binding_under(&levels);
        assert_eq!(shown, (vec![Severity::Warning], true));
    }

    #[test]
    fn warn_prints_the_lint_as_a_warning_and_passes_knave_lint() {
        let shown = LevelsFixture::unused_binding_under(&LevelsFixture::levels(LintLevel::Warn));
        assert_eq!(shown, (vec![Severity::Warning], false));
    }

    #[test]
    fn allow_hides_the_lint_and_passes_knave_lint() {
        let shown = LevelsFixture::unused_binding_under(&LevelsFixture::levels(LintLevel::Allow));
        assert_eq!(shown, (vec![], false));
    }

    #[test]
    fn deny_prints_the_lint_as_an_error_and_fails_knave_lint() {
        let shown = LevelsFixture::unused_binding_under(&LevelsFixture::levels(LintLevel::Deny));
        assert_eq!(shown, (vec![Severity::Error], true));
    }

    #[test]
    fn a_warning_that_is_no_lint_keeps_its_severity_and_passes_knave_lint() {
        let warnings = LevelsFixture::warnings(DiagnosticCode::SECOND_TRAIT_PATH);
        let levels = LevelsFixture::levels(LintLevel::Deny);
        assert_eq!(
            LevelsFixture::shown(&levels, &warnings),
            (vec![Severity::Warning], false)
        );
        let unleveled = LintLevels::default();
        assert_eq!(
            LevelsFixture::shown(&unleveled, &warnings),
            (vec![Severity::Warning], false)
        );
    }

    #[test]
    fn deny_hands_back_the_denied_lints() {
        let mut warnings = LevelsFixture::warnings(DiagnosticCode::UNUSED_BINDING);
        warnings.extend(LevelsFixture::warnings(DiagnosticCode::INPUT_SET_ASIDE));
        let levels = LevelsFixture::levels(LintLevel::Deny);
        let denied = LeveledDiagnostics::new(&warnings, &levels)
            .denied()
            .unwrap_or_default();
        let codes: Vec<&str> = denied
            .into_iter()
            .filter_map(Diagnostic::code)
            .map(DiagnosticCode::as_str)
            .collect();
        assert_eq!(codes, ["L0801"]);
    }

    #[test]
    fn every_other_level_denies_nothing() {
        let warnings = LevelsFixture::warnings(DiagnosticCode::UNUSED_BINDING);
        for level in [LintLevel::Warn, LintLevel::Allow] {
            let levels = LevelsFixture::levels(level);
            assert!(
                LeveledDiagnostics::new(&warnings, &levels)
                    .denied()
                    .is_none()
            );
        }
        assert!(
            LeveledDiagnostics::new(&warnings, &LintLevels::default())
                .denied()
                .is_none()
        );
    }

    #[test]
    fn no_levels_hand_back_the_diagnostics_they_were_given() {
        let warnings = LevelsFixture::warnings(DiagnosticCode::UNUSED_BINDING);
        let levels = LintLevels::default();
        let leveled = LeveledDiagnostics::new(&warnings, &levels);
        assert!(matches!(leveled.shown(), Cow::Borrowed(shown) if std::ptr::eq(shown, &warnings)));
    }

    #[test]
    fn applying_levels_keeps_the_order_of_the_diagnostics() {
        let mut warnings = LevelsFixture::warnings(DiagnosticCode::UNUSED_BINDING);
        warnings.push(LevelsFixture::warning(DiagnosticCode::UNREACHABLE_ARM, 5));
        warnings.extend(LevelsFixture::warnings(DiagnosticCode::UNUSED_PARAMETER));
        let levels = LevelsFixture::levels(LintLevel::Warn);
        let shown = LeveledDiagnostics::new(&warnings, &levels).shown();
        let codes: Vec<&str> = shown
            .into_iter()
            .filter_map(Diagnostic::code)
            .map(DiagnosticCode::as_str)
            .collect();
        assert_eq!(codes, ["L0801", "L0803", "L0806"]);
    }
}

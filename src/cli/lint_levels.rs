use rustc_hash::FxHashMap;

use super::lint_setting::LintSetting;
use crate::diagnostic::{CodeStage, Diagnostic, DiagnosticCode};
use crate::lint::LintDefaults;
use crate::project::{LintLevel, LintTable};

#[derive(Clone, Default)]
pub struct LintLevels {
    levels: FxHashMap<DiagnosticCode, LintLevel>,
}

impl LintLevels {
    pub fn of(table: &LintTable) -> Self {
        table
            .levels()
            .map(|(code, level)| (code.clone(), level))
            .collect()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }

    pub(super) fn setting_of(&self, diagnostic: &Diagnostic) -> Option<LintSetting> {
        let code = diagnostic
            .code()
            .filter(|code| CodeStage::LINTS.contains(code))?;
        Some(
            self.level_of(code)
                .map_or(LintSetting::Unset, LintSetting::Level),
        )
    }

    pub(super) fn level_of(&self, code: &DiagnosticCode) -> Option<LintLevel> {
        LintDefaults::level_of(code, self.levels.get(code).copied())
    }
}

impl FromIterator<(DiagnosticCode, LintLevel)> for LintLevels {
    fn from_iter<Levels: IntoIterator<Item = (DiagnosticCode, LintLevel)>>(levels: Levels) -> Self {
        Self {
            levels: levels.into_iter().collect(),
        }
    }
}

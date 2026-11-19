use crate::project::LintLevel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LintSetting {
    Unset,
    Level(LintLevel),
}

#[derive(Debug, PartialEq, Eq)]
pub struct LintBehavior {
    pub shown: bool,
    pub denied: bool,
    pub fails_lint: bool,
}

impl LintSetting {
    pub fn behavior(self) -> LintBehavior {
        match self {
            Self::Unset => LintBehavior {
                shown: true,
                denied: false,
                fails_lint: true,
            },
            Self::Level(LintLevel::Warn) => LintBehavior {
                shown: true,
                denied: false,
                fails_lint: false,
            },
            Self::Level(LintLevel::Allow) => LintBehavior {
                shown: false,
                denied: false,
                fails_lint: false,
            },
            Self::Level(LintLevel::Deny) => LintBehavior {
                shown: true,
                denied: true,
                fails_lint: true,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LintLevel {
    Allow,
    Warn,
    Deny,
}

impl LintLevel {
    pub(super) const ALL: [Self; 3] = [Self::Allow, Self::Warn, Self::Deny];

    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Allow => "silent-level",
            Self::Warn => "warning-level",
            Self::Deny => "error-level",
        }
    }
}

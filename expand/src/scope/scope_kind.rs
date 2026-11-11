#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum ScopeKind {
    Module,
    Local,
    UseSite,
    Introduction,
}

impl ScopeKind {
    pub fn is_stripped_by_quote(self) -> bool {
        matches!(self, Self::Local | Self::UseSite)
    }
}

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

#[cfg(test)]
mod tests {
    use super::ScopeKind;

    #[test]
    fn a_quote_strips_local_and_use_site_scopes() {
        assert!(ScopeKind::Local.is_stripped_by_quote());
        assert!(ScopeKind::UseSite.is_stripped_by_quote());
    }

    #[test]
    fn a_quote_keeps_module_and_introduction_scopes() {
        assert!(!ScopeKind::Module.is_stripped_by_quote());
        assert!(!ScopeKind::Introduction.is_stripped_by_quote());
    }
}

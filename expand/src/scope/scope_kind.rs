#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) enum ScopeKind {
    Module,
    Local,
    UseSite,
    Introduction,
}

impl ScopeKind {
    pub(crate) fn is_stripped_by_quote(self) -> bool {
        matches!(self, Self::Local | Self::UseSite)
    }

    pub(crate) fn is_introduction(self) -> bool {
        self == Self::Introduction
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

    #[test]
    fn only_an_introduction_scope_is_one() {
        assert!(ScopeKind::Introduction.is_introduction());
        for kind in [ScopeKind::Module, ScopeKind::Local, ScopeKind::UseSite] {
            assert!(!kind.is_introduction(), "{kind:?}");
        }
    }
}

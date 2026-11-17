use std::path::Path;

use super::search_scope::SearchScope;

pub(super) struct LocalRoot<'s> {
    root: &'s Path,
    display_root: &'s Path,
    scope: &'s SearchScope,
    role: RootRole,
}

#[derive(Clone, Copy, PartialEq)]
enum RootRole {
    Primary,
    Fallback,
}

impl<'s> LocalRoot<'s> {
    pub(super) fn primary(root: &'s Path, display_root: &'s Path, scope: &'s SearchScope) -> Self {
        Self {
            root,
            display_root,
            scope,
            role: RootRole::Primary,
        }
    }

    pub(super) fn fallback(root: &'s Path, scope: &'s SearchScope) -> Self {
        Self {
            root,
            display_root: root,
            scope,
            role: RootRole::Fallback,
        }
    }

    pub(super) fn root(&self) -> &'s Path {
        self.root
    }

    pub(super) fn display_root(&self) -> &'s Path {
        self.display_root
    }

    pub(super) fn is_same_root(&self, other: &LocalRoot<'_>) -> bool {
        self.role == other.role
    }

    pub(super) fn reaches(&self, local: &Path) -> bool {
        let path = self.root.join(local);
        path.exists() && self.allows(&path)
    }

    pub(super) fn holds_unreachable(&self, local: &Path) -> bool {
        let path = self.root.join(local);
        path.exists() && !self.allows(&path)
    }

    fn allows(&self, path: &Path) -> bool {
        match self.role {
            RootRole::Primary => self.scope.primary_reaches(path),
            RootRole::Fallback => self.scope.fallback_reaches(path),
        }
    }
}

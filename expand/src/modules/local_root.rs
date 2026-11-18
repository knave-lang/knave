use std::fs;
use std::path::{Component, Path, PathBuf};

use super::search_scope::SearchScope;

pub(super) struct LocalRoot<'s> {
    root: &'s Path,
    display_root: &'s Path,
    scope: &'s SearchScope,
    role: RootRole,
}

pub(super) struct ReachedFile {
    pub(super) path: PathBuf,
    pub(super) shown: PathBuf,
}

#[derive(Clone, Copy)]
enum RootRole {
    Primary,
    Fallback,
}

impl<'s> LocalRoot<'s> {
    const CURRENT_DIRECTORY: &'static str = ".";

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

    pub(super) fn reached_file(&self, local: &Path) -> Option<ReachedFile> {
        let path = self.root.join(local);
        let reached = path.exists() && self.allows(&path);
        reached.then(|| ReachedFile {
            path,
            shown: self.display_root.join(local),
        })
    }

    pub(super) fn unreachable_file(&self, local: &Path) -> Option<PathBuf> {
        let path = self.root.join(local);
        fs::canonicalize(&path).ok().filter(|_| !self.allows(&path))
    }

    pub(super) fn shown_file(&self, local: &Path, found: &Path) -> PathBuf {
        let shown = Self::normalized(&self.display_root.join(local));
        let shows_found =
            !shown.is_symlink() && fs::canonicalize(&shown).is_ok_and(|file| file == found);
        if shows_found {
            shown
        } else {
            found.to_path_buf()
        }
    }

    pub(super) fn canonical_file(&self, local: &Path) -> Option<PathBuf> {
        let path = self.root.join(local);
        let file = fs::canonicalize(path.parent()?)
            .ok()?
            .join(path.file_name()?);
        file.is_file().then_some(file)
    }

    pub(super) fn local_part<'f>(&self, canonical_file: &'f Path) -> Option<&'f Path> {
        let directory = Path::new(Self::CURRENT_DIRECTORY).join(self.root);
        let canonical_root = fs::canonicalize(directory).ok()?;
        canonical_file.strip_prefix(canonical_root).ok()
    }

    fn allows(&self, path: &Path) -> bool {
        match self.role {
            RootRole::Primary => self.scope.primary_reaches(path),
            RootRole::Fallback => self.scope.fallback_reaches(path),
        }
    }

    fn normalized(path: &Path) -> PathBuf {
        let mut normalized = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir if normalized.file_name().is_some() => {
                    normalized.pop();
                }
                other => normalized.push(other),
            }
        }
        normalized
    }
}

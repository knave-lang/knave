use std::path::{Component, Path, PathBuf};

use super::local_root::LocalRoot;
use crate::core::{DottedModulePath, ModulePath};
use crate::language::Language;

pub struct LocalFiles<'s> {
    roots: Vec<LocalRoot<'s>>,
}

pub enum DottedPathTarget {
    Reachable(ModulePath),
    Unreachable(PathBuf),
    Absent,
}

impl<'s> LocalFiles<'s> {
    pub(super) fn new(roots: Vec<LocalRoot<'s>>) -> Self {
        Self { roots }
    }

    pub fn unreachable_file(&self, path: &ModulePath) -> Option<PathBuf> {
        let file_name = Language::file_name(path.as_str());
        self.roots
            .iter()
            .find(|root| root.holds_unreachable(Path::new(&file_name)))
            .map(|root| Self::normalized(&root.display_root().join(&file_name)))
    }

    pub fn dotted_path_target(&self, path: &DottedModulePath) -> DottedPathTarget {
        let file_name = Language::file_name(path.as_str());
        let Some(holder) = self.roots.iter().find(|root| Self::holds(root, &file_name)) else {
            return DottedPathTarget::Absent;
        };
        let file = Self::normalized(&holder.root().join(&file_name));
        match self
            .roots
            .iter()
            .find_map(|root| self.written_path(root, &file))
        {
            Some(written) => DottedPathTarget::Reachable(written),
            None => DottedPathTarget::Unreachable(Self::normalized(
                &holder.display_root().join(&file_name),
            )),
        }
    }

    fn holds(root: &LocalRoot<'_>, file_name: &str) -> bool {
        Self::normalized(&root.root().join(file_name)).is_file()
    }

    fn written_path(&self, root: &LocalRoot<'_>, file: &Path) -> Option<ModulePath> {
        let local = file.strip_prefix(Self::normalized(root.root())).ok()?;
        let first_reach = self
            .roots
            .iter()
            .find(|candidate| candidate.reaches(local))?;
        first_reach
            .is_same_root(root)
            .then(|| Self::module_path(local))
    }

    fn module_path(local: &Path) -> ModulePath {
        let names: Vec<_> = local
            .with_extension("")
            .components()
            .map(|name| name.as_os_str().to_string_lossy().into_owned())
            .collect();
        ModulePath::new(names.join(ModulePath::SEPARATOR))
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

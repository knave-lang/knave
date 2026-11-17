use std::path::{Path, PathBuf};

use super::dotted_path_target::DottedPathTarget;
use super::local_root::LocalRoot;
use crate::core::{DottedModulePath, ModulePath};
use crate::language::Language;

pub struct LocalFiles<'s> {
    roots: Vec<LocalRoot<'s>>,
    loads_elsewhere: Box<dyn Fn(&ModulePath) -> bool + 's>,
}

impl<'s> LocalFiles<'s> {
    pub(super) fn new(
        roots: Vec<LocalRoot<'s>>,
        loads_elsewhere: impl Fn(&ModulePath) -> bool + 's,
    ) -> Self {
        Self {
            roots,
            loads_elsewhere: Box::new(loads_elsewhere),
        }
    }

    pub fn unreachable_file(&self, path: &ModulePath) -> Option<PathBuf> {
        let file_name = Language::file_name(path.as_str());
        let local = Path::new(&file_name);
        self.roots
            .iter()
            .find(|root| root.holds_unreachable(local))
            .map(|root| root.shown_file(local))
    }

    pub fn dotted_path_target(&self, path: &DottedModulePath) -> DottedPathTarget {
        let file_name = Language::file_name(path.as_str());
        let local = Path::new(&file_name);
        let Some((holder, file)) = self
            .roots
            .iter()
            .find_map(|root| Some((root, root.canonical_file(local)?)))
        else {
            return DottedPathTarget::Absent;
        };
        match self
            .roots
            .iter()
            .find_map(|root| self.written_path(root, &file))
        {
            Some(written) => DottedPathTarget::Reachable(written),
            None => DottedPathTarget::Unreachable(holder.shown_file(local)),
        }
    }

    fn written_path(&self, root: &LocalRoot<'_>, file: &Path) -> Option<ModulePath> {
        let local = root.local_part(file)?;
        let first_reach = self
            .roots
            .iter()
            .find(|candidate| candidate.reached_file(local).is_some())?;
        let written = ModulePath::of_file(local)?;
        let loads_here = std::ptr::eq(first_reach, root) && !(self.loads_elsewhere)(&written);
        loads_here.then_some(written)
    }
}

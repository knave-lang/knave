use std::path::{Path, PathBuf};

use super::local_root::LocalRoot;
use super::unnamed_part_target::UnnamedPartTarget;
use crate::core::{ModulePath, UnnamedPartPath};
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

    pub fn unnamed_part_target(&self, path: &UnnamedPartPath) -> UnnamedPartTarget {
        if let Some(written) = self.written_in_place(path) {
            return UnnamedPartTarget::Reachable(written);
        }
        let file_name = Language::file_name(path.as_str());
        let local = Path::new(&file_name);
        let Some((holder, file)) = self
            .roots
            .iter()
            .find_map(|root| Some((root, root.canonical_file(local)?)))
        else {
            return UnnamedPartTarget::Absent;
        };
        match (0..self.roots.len()).find_map(|position| self.written_path(position, &file)) {
            Some(written) => UnnamedPartTarget::Reachable(written),
            None => self.target_without_module_path(holder, local),
        }
    }

    fn written_in_place(&self, path: &UnnamedPartPath) -> Option<ModulePath> {
        let written = path.named_in_place()?;
        let file_name = Language::file_name(written.as_str());
        let reached = self
            .roots
            .iter()
            .any(|root| root.reached_file(Path::new(&file_name)).is_some());
        (reached && !(self.loads_elsewhere)(&written)).then_some(written)
    }

    fn target_without_module_path(
        &self,
        holder: &LocalRoot<'_>,
        local: &Path,
    ) -> UnnamedPartTarget {
        match self.roots.iter().find(|root| root.holds_unreachable(local)) {
            Some(scoped) => UnnamedPartTarget::BeyondTaskReach(scoped.shown_file(local)),
            None => UnnamedPartTarget::Unreachable(holder.shown_file(local)),
        }
    }

    fn written_path(&self, position: usize, file: &Path) -> Option<ModulePath> {
        let local = self.roots[position].local_part(file)?;
        let first_reach = self
            .roots
            .iter()
            .position(|root| root.reached_file(local).is_some())?;
        let written = ModulePath::of_file(local)?;
        let names_the_file = Path::new(&Language::file_name(written.as_str())) == local;
        let loads_here =
            names_the_file && first_reach == position && !(self.loads_elsewhere)(&written);
        loads_here.then_some(written)
    }
}

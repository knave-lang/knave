use std::path::{Path, PathBuf};

use super::local_root::LocalRoot;
use super::other_libraries::OtherLibraries;
use super::unnamed_part_target::UnnamedPartTarget;
use super::visible_library::VisibleLibrary;
use crate::core::{ModulePath, UnnamedPartPath};
use crate::language::Language;

pub struct LocalFiles<'s> {
    roots: Vec<LocalRoot<'s>>,
    other_libraries: OtherLibraries<'s>,
    loads_elsewhere: Box<dyn Fn(&ModulePath) -> bool + 's>,
}

enum FileOwner<'f, 's> {
    LocalRoot,
    Library(&'f VisibleLibrary<'s>, PathBuf),
}

impl<'s> LocalFiles<'s> {
    pub(super) fn new(
        roots: Vec<LocalRoot<'s>>,
        other_libraries: OtherLibraries<'s>,
        loads_elsewhere: impl Fn(&ModulePath) -> bool + 's,
    ) -> Self {
        Self {
            roots,
            other_libraries,
            loads_elsewhere: Box::new(loads_elsewhere),
        }
    }

    pub fn unreachable_file(&self, path: &ModulePath) -> Option<PathBuf> {
        let file_name = Language::file_name(path.as_str());
        let local = Path::new(&file_name);
        self.roots
            .iter()
            .find_map(|root| Some(root.shown_file(local, &root.unreachable_file(local)?)))
    }

    pub fn unnamed_part_target(&self, path: &UnnamedPartPath) -> UnnamedPartTarget<'s> {
        let file_name = Language::file_name(path.as_str());
        let local = Path::new(&file_name);
        let found = self.found_file(local);
        let file = found
            .as_ref()
            .map(|(_, file)| file.clone())
            .or_else(|| self.file_in_place(path));
        let owner = file.as_deref().and_then(|file| self.innermost_owner(file));
        match (owner, file) {
            (Some(FileOwner::Library(holder, inner)), Some(file)) => {
                self.other_libraries.target(holder, &inner, &file)
            }
            (owner, _) => {
                let written = self
                    .owned_written_path(found.as_ref(), owner)
                    .or_else(|| self.written_in_place(path));
                self.local_target(written, local, found)
            }
        }
    }

    fn found_file(&self, local: &Path) -> Option<(&LocalRoot<'s>, PathBuf)> {
        self.roots
            .iter()
            .find_map(|root| Some((root, root.canonical_file(local)?)))
    }

    fn file_in_place(&self, path: &UnnamedPartPath) -> Option<PathBuf> {
        let file_name = Language::file_name(path.named_in_place()?.as_str());
        self.roots
            .iter()
            .find_map(|root| root.canonical_file(Path::new(&file_name)))
    }

    fn innermost_owner(&self, file: &Path) -> Option<FileOwner<'_, 's>> {
        let local_roots = self
            .roots
            .iter()
            .filter_map(|root| Some((FileOwner::LocalRoot, root.local_part(file)?.to_path_buf())));
        let libraries = self
            .other_libraries
            .holders(file)
            .into_iter()
            .map(|(holder, inner)| (FileOwner::Library(holder, inner.clone()), inner));
        local_roots
            .chain(libraries)
            .min_by_key(|(_, inner)| Self::depth_below_root(inner))
            .map(|(owner, _)| owner)
    }

    fn owned_written_path(
        &self,
        found: Option<&(&LocalRoot<'s>, PathBuf)>,
        owner: Option<FileOwner<'_, 's>>,
    ) -> Option<ModulePath> {
        match (found, owner) {
            (Some((_, file)), Some(FileOwner::LocalRoot)) => self.innermost_written_path(file),
            _ => None,
        }
    }

    fn innermost_written_path(&self, file: &Path) -> Option<ModulePath> {
        (0..self.roots.len())
            .filter_map(|position| self.written_path(position, file))
            .min_by_key(|written| Self::depth_below_root(Path::new(written.as_str())))
    }

    fn depth_below_root(inner: &Path) -> usize {
        inner.components().count()
    }

    fn local_target(
        &self,
        written: Option<ModulePath>,
        local: &Path,
        found: Option<(&LocalRoot<'s>, PathBuf)>,
    ) -> UnnamedPartTarget<'s> {
        match (written, found) {
            (Some(written), _) => UnnamedPartTarget::Reachable(written),
            (None, Some((holder, file))) => self.target_without_module_path(holder, local, &file),
            (None, None) => UnnamedPartTarget::Absent,
        }
    }

    fn written_in_place(&self, path: &UnnamedPartPath) -> Option<ModulePath> {
        let written = path.named_in_place()?;
        let file_name = Language::file_name(written.as_str());
        let local = Path::new(&file_name);
        let reached = self
            .roots
            .iter()
            .any(|root| root.reached_file(local).is_some());
        (reached && !(self.loads_elsewhere)(&written)).then_some(written)
    }

    fn target_without_module_path(
        &self,
        holder: &LocalRoot<'_>,
        local: &Path,
        file: &Path,
    ) -> UnnamedPartTarget<'s> {
        match self
            .roots
            .iter()
            .find_map(|root| Some(root.shown_file(local, &root.unreachable_file(local)?)))
        {
            Some(shown) => UnnamedPartTarget::BeyondTaskReach(shown),
            None => UnnamedPartTarget::Unreachable(holder.shown_file(local, file)),
        }
    }

    fn written_path(&self, position: usize, file: &Path) -> Option<ModulePath> {
        let local = self.roots[position].local_part(file)?;
        let first_reach = self
            .roots
            .iter()
            .position(|root| root.reached_file(local).is_some())?;
        let written = ModulePath::of_file(local)?;
        let loads_here = first_reach == position && !(self.loads_elsewhere)(&written);
        loads_here.then_some(written)
    }
}

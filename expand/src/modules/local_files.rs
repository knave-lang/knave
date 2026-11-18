use std::path::{Path, PathBuf};

use super::local_root::LocalRoot;
use super::unlisted_library::UnlistedLibrary;
use super::unnamed_part_target::UnnamedPartTarget;
use super::visible_library::VisibleLibrary;
use crate::core::{ModulePath, UnnamedPartPath};
use crate::language::Language;

pub struct LocalFiles<'s> {
    roots: Vec<LocalRoot<'s>>,
    loads_elsewhere: Box<dyn Fn(&ModulePath) -> bool + 's>,
    other_libraries: Vec<VisibleLibrary<'s>>,
    library_section: &'s str,
}

impl<'s> LocalFiles<'s> {
    pub(super) fn new(
        roots: Vec<LocalRoot<'s>>,
        loads_elsewhere: impl Fn(&ModulePath) -> bool + 's,
    ) -> Self {
        Self {
            roots,
            loads_elsewhere: Box::new(loads_elsewhere),
            other_libraries: Vec::new(),
            library_section: "",
        }
    }

    pub(super) fn beside(
        self,
        other_libraries: Vec<VisibleLibrary<'s>>,
        library_section: &'s str,
    ) -> Self {
        Self {
            other_libraries,
            library_section,
            ..self
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
        let reached = found.as_ref().map(|(_, file)| file.clone());
        reached
            .or_else(|| self.file_in_place(path))
            .and_then(|file| self.library_target(&file))
            .unwrap_or_else(|| self.local_target(path, local, found))
    }

    fn found_file(&self, local: &Path) -> Option<(&LocalRoot<'s>, PathBuf)> {
        self.roots
            .iter()
            .find_map(|root| Some((root, root.canonical_file(local)?)))
    }

    fn local_target(
        &self,
        path: &UnnamedPartPath,
        local: &Path,
        found: Option<(&LocalRoot<'s>, PathBuf)>,
    ) -> UnnamedPartTarget<'s> {
        let written = found
            .as_ref()
            .and_then(|(_, file)| self.innermost_written_path(file))
            .or_else(|| self.written_in_place(path));
        match (written, found) {
            (Some(written), _) => UnnamedPartTarget::Reachable(written),
            (None, Some((holder, file))) => self.target_without_module_path(holder, local, &file),
            (None, None) => UnnamedPartTarget::Absent,
        }
    }

    fn file_in_place(&self, path: &UnnamedPartPath) -> Option<PathBuf> {
        let file_name = Language::file_name(path.named_in_place()?.as_str());
        self.roots
            .iter()
            .find_map(|root| root.canonical_file(Path::new(&file_name)))
    }

    fn library_target(&self, file: &Path) -> Option<UnnamedPartTarget<'s>> {
        let (holder, inner) = self
            .other_libraries
            .iter()
            .filter_map(|holder| Some((holder, holder.root.local_part(file)?)))
            .min_by_key(|(_, inner)| Self::depth_below_root(inner))?;
        let local_depth = self
            .roots
            .iter()
            .filter_map(|root| root.local_part(file))
            .map(Self::depth_below_root)
            .min();
        if local_depth.is_some_and(|depth| depth <= Self::depth_below_root(inner)) {
            return None;
        }
        let written = holder
            .visible
            .then(|| holder.library.module_path_of(inner))
            .flatten();
        Some(match written {
            Some(written) => UnnamedPartTarget::Reachable(written),
            None => UnnamedPartTarget::LibraryFile {
                file: holder.root.shown_file(inner, file),
                unlisted: UnlistedLibrary {
                    name: holder.library.name(),
                    lister: None,
                    section: self.library_section,
                },
            },
        })
    }

    fn innermost_written_path(&self, file: &Path) -> Option<ModulePath> {
        (0..self.roots.len())
            .filter_map(|position| self.written_path(position, file))
            .min_by_key(|written| Self::depth_below_root(Path::new(written.as_str())))
    }

    fn depth_below_root(inner: &Path) -> usize {
        inner.components().count()
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

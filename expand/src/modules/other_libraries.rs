use std::path::{Path, PathBuf};

use super::unlisted_library::UnlistedLibrary;
use super::unnamed_part_target::UnnamedPartTarget;
use super::visible_library::VisibleLibrary;

pub(super) struct OtherLibraries<'s> {
    libraries: Vec<VisibleLibrary<'s>>,
    section: &'s str,
}

impl<'s> OtherLibraries<'s> {
    pub(super) fn new(libraries: Vec<VisibleLibrary<'s>>, section: &'s str) -> Self {
        Self { libraries, section }
    }

    pub(super) fn holders(&self, file: &Path) -> Vec<(&VisibleLibrary<'s>, PathBuf)> {
        self.libraries
            .iter()
            .filter_map(|holder| Some((holder, holder.root.local_part(file)?.to_path_buf())))
            .collect()
    }

    pub(super) fn target(
        &self,
        holder: &VisibleLibrary<'s>,
        inner: &Path,
        file: &Path,
    ) -> UnnamedPartTarget<'s> {
        let written = holder
            .visible
            .then(|| holder.library.module_path_of(inner))
            .flatten();
        match written {
            Some(written) => UnnamedPartTarget::Reachable(written),
            None => UnnamedPartTarget::LibraryFile {
                file: holder.root.shown_file(inner, file),
                unlisted: UnlistedLibrary {
                    name: holder.library.name(),
                    lister: None,
                    section: self.section,
                },
            },
        }
    }
}

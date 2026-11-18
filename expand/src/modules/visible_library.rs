use std::path::Path;

use super::local_root::LocalRoot;
use crate::core::ModulePath;

pub(super) trait LibraryNaming {
    fn name(&self) -> &str;
    fn module_path_of(&self, inner: &Path) -> Option<ModulePath>;
}

pub(super) struct VisibleLibrary<'s> {
    pub(super) library: &'s dyn LibraryNaming,
    pub(super) root: LocalRoot<'s>,
    pub(super) visible: bool,
    pub(super) section: &'s str,
}

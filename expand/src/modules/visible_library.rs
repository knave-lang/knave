use super::library_naming::LibraryNaming;
use super::local_root::LocalRoot;

pub(super) struct VisibleLibrary<'s> {
    pub(super) library: &'s dyn LibraryNaming,
    pub(super) root: LocalRoot<'s>,
    pub(super) visible: bool,
}

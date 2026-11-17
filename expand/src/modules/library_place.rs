use std::path::Path;

pub(super) struct LibraryPlace<'s> {
    pub(super) name: &'s str,
    pub(super) source_root: &'s Path,
    pub(super) display_root: &'s Path,
    pub(super) root_module: &'s str,
    pub(super) visible: bool,
}

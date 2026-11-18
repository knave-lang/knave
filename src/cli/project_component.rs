use std::path::Path;

use super::new_file::NewFile;

pub trait ProjectComponent {
    fn files(&self, root: &Path) -> Vec<NewFile>;

    fn set_up(&self, _root: &Path) {}
}

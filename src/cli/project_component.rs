use std::path::Path;

use super::new_file::NewFile;
use super::owned_file::OwnedFile;

pub trait ProjectComponent {
    fn owned_files(&self) -> Vec<OwnedFile> {
        Vec::new()
    }

    fn files(&self, root: &Path) -> Vec<NewFile> {
        self.owned_files()
            .iter()
            .map(|file| file.new_file(root))
            .collect()
    }

    fn set_up(&self, _root: &Path) {}
}

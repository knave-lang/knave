use std::path::Path;

use super::new_file::NewFile;
use super::owned_file::OwnedFile;
use super::owned_line::OwnedLine;
use super::pending_writes::PendingWrites;

pub trait ProjectComponent {
    fn owned_files(&self) -> Vec<OwnedFile> {
        Vec::new()
    }

    fn owned_lines(&self) -> Vec<OwnedLine> {
        Vec::new()
    }

    fn existing_files(&self, root: &Path) -> Vec<OwnedFile> {
        self.owned_files()
            .into_iter()
            .filter(|file| file.exists_in(root))
            .collect()
    }

    fn shows(&self, root: &Path) -> bool {
        !self.existing_files(root).is_empty()
    }

    fn pending_writes(&self, root: &Path) -> PendingWrites {
        let files = self
            .owned_files()
            .into_iter()
            .map(|file| file.pending_write(root));
        let lines = self
            .owned_lines()
            .into_iter()
            .map(|line| line.pending_write(root));
        files.chain(lines).collect()
    }

    fn files(&self, root: &Path) -> Vec<NewFile> {
        let files = self
            .owned_files()
            .into_iter()
            .map(|file| file.new_file(root));
        let lines = self
            .owned_lines()
            .into_iter()
            .map(|line| line.new_file(root));
        files.chain(lines).collect()
    }

    fn set_up(&self, _root: &Path) {}
}

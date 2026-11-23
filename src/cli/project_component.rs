use std::path::Path;

use super::github_linguist::GithubLinguist;
use super::new_file::NewFile;
use super::owned_file::OwnedFile;

pub trait ProjectComponent {
    fn owned_files(&self) -> Vec<OwnedFile> {
        Vec::new()
    }

    fn owned_lines(&self) -> Vec<GithubLinguist> {
        Vec::new()
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

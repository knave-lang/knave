use std::fs;
use std::path::{Path, PathBuf};

use crate::failure::TaskFailure;
use crate::git::Git;

pub struct RevisionTree {
    directory: PathBuf,
    root: PathBuf,
}

impl RevisionTree {
    const PARENT: &str = "target/xtask-revision-trees";
    const INDEX: &str = "index";
    const TREE: &str = "tree";
    const PREFIX_SEPARATOR: char = '/';

    pub fn extract(workspace: &Path, revision: &str) -> Result<Self, TaskFailure> {
        let directory = workspace
            .join(Self::PARENT)
            .join(format!("{revision}-{}", std::process::id()));
        fs::create_dir_all(&directory)
            .map_err(|error| TaskFailure::unwritable(&directory, error))?;
        let tree = Self {
            root: directory.join(Self::TREE),
            directory,
        };
        tree.check_out(&Git::new(workspace), revision)?;
        Ok(tree)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn check_out(&self, git: &Git, revision: &str) -> Result<(), TaskFailure> {
        let index = self.directory.join(Self::INDEX);
        let prefix = format!("--prefix={}{}", self.root.display(), Self::PREFIX_SEPARATOR);
        git.succeed_with_index(&index, &["read-tree", revision])?;
        git.succeed_with_index(&index, &["checkout-index", "--all", &prefix])?;
        Ok(())
    }
}

impl Drop for RevisionTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::RevisionTree;
    use crate::temporary_repository::TemporaryRepository;

    #[test]
    fn the_tree_holds_the_revision_and_leaves_the_working_tree() {
        let repository = TemporaryRepository::initialized("revision-tree");
        repository.write("counts.txt", "first");
        repository.commit_all();
        repository.write("counts.txt", "second");
        let tree = RevisionTree::extract(repository.root(), "HEAD").expect("it extracts");
        let text = fs::read_to_string(tree.root().join("counts.txt")).expect("the file reads");
        assert_eq!(text, "first");
    }

    #[test]
    fn dropping_the_tree_removes_its_files() {
        let repository = TemporaryRepository::initialized("revision-tree-drop");
        repository.write("counts.txt", "first");
        repository.commit_all();
        let tree = RevisionTree::extract(repository.root(), "HEAD").expect("it extracts");
        let root = tree.root().to_path_buf();
        drop(tree);
        assert!(!root.exists());
    }

    #[test]
    fn an_unknown_revision_fails() {
        let repository = TemporaryRepository::initialized("revision-tree-unknown");
        let failure = RevisionTree::extract(repository.root(), "no-such-revision")
            .err()
            .expect("the revision is unknown");
        assert!(failure.to_string().contains("git read-tree"), "{failure}");
    }
}

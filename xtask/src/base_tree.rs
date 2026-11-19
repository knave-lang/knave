use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::failure::TaskFailure;
use crate::git::Git;

static EXTRACTED_TREES: AtomicUsize = AtomicUsize::new(0);

pub struct BaseTree {
    directory: PathBuf,
    root: PathBuf,
}

impl BaseTree {
    const INDEX: &str = "index";
    const TREE: &str = "tree";
    const PREFIX_SEPARATOR: char = '/';

    pub fn extract(git: &Git, revision: &str) -> Result<Self, TaskFailure> {
        let directory = Self::fresh_directory();
        fs::create_dir_all(&directory)
            .map_err(|error| TaskFailure::unwritable(&directory, error))?;
        let tree = Self {
            root: directory.join(Self::TREE),
            directory,
        };
        tree.check_out(git, revision)?;
        Ok(tree)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn fresh_directory() -> PathBuf {
        let sequence = EXTRACTED_TREES.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("xtask-base-{}-{sequence}", std::process::id()))
    }

    fn check_out(&self, git: &Git, revision: &str) -> Result<(), TaskFailure> {
        let index = self.directory.join(Self::INDEX);
        let prefix = format!("--prefix={}{}", self.root.display(), Self::PREFIX_SEPARATOR);
        git.succeed_with_index(&index, &["read-tree", revision])?;
        git.succeed_with_index(&index, &["checkout-index", "--all", &prefix])?;
        Ok(())
    }
}

impl Drop for BaseTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::BaseTree;
    use crate::git::Git;
    use crate::temporary_repository::TemporaryRepository;

    #[test]
    fn the_tree_holds_the_revision_and_leaves_the_working_tree() {
        let repository = TemporaryRepository::initialized("base-tree");
        repository.write("counts.txt", "first");
        repository.commit_all();
        repository.write("counts.txt", "second");
        let tree = BaseTree::extract(&Git::new(repository.root()), "HEAD").expect("it extracts");
        let text = fs::read_to_string(tree.root().join("counts.txt")).expect("the file reads");
        assert_eq!(text, "first");
        let directory = tree.directory.clone();
        drop(tree);
        assert!(!directory.exists());
    }

    #[test]
    fn an_unknown_revision_fails() {
        let repository = TemporaryRepository::initialized("base-tree-unknown");
        let failure = BaseTree::extract(&Git::new(repository.root()), "no-such-revision")
            .err()
            .expect("the revision is unknown");
        assert!(failure.to_string().contains("git read-tree"), "{failure}");
    }
}

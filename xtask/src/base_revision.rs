use std::path::Path;

use clap::Args;

use crate::failure::TaskFailure;
use crate::git::Git;

#[derive(Args, Debug, PartialEq)]
pub struct BaseRevision {
    #[arg(
        long = "base",
        default_value = "origin/main",
        help = "Judge the changes since the branch forked from this revision"
    )]
    revision: String,
}

impl BaseRevision {
    #[cfg(test)]
    pub fn named(revision: &str) -> Self {
        Self {
            revision: revision.to_string(),
        }
    }

    pub fn fork_point(&self, root: &Path) -> Result<String, TaskFailure> {
        Git::new(root).merge_base(&self.revision)
    }
}

#[cfg(test)]
mod tests {
    use super::BaseRevision;
    use crate::workspace::WorkspaceFixture;

    #[test]
    fn an_unknown_revision_has_no_fork_point() {
        let failure = match BaseRevision::named("no-such-revision")
            .fork_point(WorkspaceFixture::repository_root())
        {
            Ok(fork_point) => panic!("found a fork point {fork_point}"),
            Err(failure) => failure.to_string(),
        };
        assert_eq!(failure, "cannot find the revision `no-such-revision`");
    }
}

use crate::failure::TaskFailure;
use crate::git::Git;

#[derive(Debug, PartialEq)]
pub struct ForkPoint {
    commit: String,
}

impl ForkPoint {
    pub fn resolve(git: &Git, revision: &str) -> Result<Self, TaskFailure> {
        Ok(Self {
            commit: git.merge_base(revision)?,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.commit
    }
}

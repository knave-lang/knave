use std::collections::BTreeSet;

use crate::failure::TaskFailure;
use crate::git::Git;

pub struct RiseApprovals {
    trailers: Vec<String>,
}

impl RiseApprovals {
    pub const TRAILER: &str = "Baseline-Accepted";

    pub fn since(git: &Git, base: &str) -> Result<Self, TaskFailure> {
        let format = format!("--format=%(trailers:key={},valueonly)", Self::TRAILER);
        let log = git.succeed(&["log", &format, &format!("{base}..HEAD")])?;
        Ok(Self::parse(&String::from_utf8_lossy(&log)))
    }

    pub fn parse(log: &str) -> Self {
        Self {
            trailers: log.lines().map(String::from).collect(),
        }
    }

    pub fn trailer_line(limit: &str, subject: &str) -> String {
        format!("{}: {limit} {subject}", Self::TRAILER)
    }

    pub fn subjects_of(&self, limit: &str) -> BTreeSet<String> {
        self.trailers
            .iter()
            .filter_map(|trailer| {
                let mut words = trailer.split_whitespace();
                (words.next() == Some(limit)).then_some(words)
            })
            .flatten()
            .map(String::from)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::RiseApprovals;
    use crate::git::Git;
    use crate::workspace::WorkspaceFixture;

    struct ApprovalCase;

    impl ApprovalCase {
        fn subjects(log: &str, limit: &str) -> BTreeSet<String> {
            RiseApprovals::parse(log).subjects_of(limit)
        }
    }

    #[test]
    fn a_trailer_may_name_several_subjects_of_one_limit() {
        assert_eq!(
            ApprovalCase::subjects(
                "production_lines src/a.rs\n\nproduction_lines src/b.rs src/c.rs\n",
                "production_lines"
            ),
            BTreeSet::from(["src/a.rs".into(), "src/b.rs".into(), "src/c.rs".into()])
        );
    }

    #[test]
    fn a_trailer_of_another_limit_approves_nothing() {
        assert!(
            ApprovalCase::subjects(
                "cycle_links_per_module src/a.rs\nsrc/a.rs src/b.rs\n",
                "production_lines"
            )
            .is_empty()
        );
    }

    #[test]
    fn the_trailer_line_names_the_limit_and_the_subject() {
        assert_eq!(
            RiseApprovals::trailer_line("production_lines", "src/a.rs"),
            "Baseline-Accepted: production_lines src/a.rs"
        );
    }

    #[test]
    fn no_commit_since_head_approves_nothing() {
        let git = Git::new(WorkspaceFixture::repository_root());
        let approvals = RiseApprovals::since(&git, "HEAD").expect("the log reads");
        assert!(approvals.subjects_of("production_lines").is_empty());
    }

    #[test]
    fn a_failed_log_fails() {
        let git = Git::new(WorkspaceFixture::repository_root());
        assert!(RiseApprovals::since(&git, "no-such-revision").is_err());
    }
}

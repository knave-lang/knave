use std::collections::BTreeSet;
use std::path::Path;

use crate::checks::Check;
use crate::failure::TaskFailure;
use crate::git::Git;
use crate::ratchet::{Baseline, Raise};

pub struct BaselineRaises {
    file: String,
    raises: Vec<Raise>,
    accepted: BTreeSet<String>,
}

impl BaselineRaises {
    pub const TRAILER: &str = "Baseline-Accepted";

    pub fn since(root: &Path, file: &str, base: &str) -> Result<Self, TaskFailure> {
        let git = Git::new(root);
        Self::verify_revision(&git, base)?;
        let raises = match Self::file_at(&git, base, file)? {
            Some(text) => Baseline::read(root, file)?.raised_since(&Baseline::parse(file, &text)?),
            None => Vec::new(),
        };
        let log = Self::trailer_log(&git, base)?;
        Ok(Self::new(file, raises, Self::accepted_subjects(&log)))
    }

    fn new(file: &str, raises: Vec<Raise>, accepted: BTreeSet<String>) -> Self {
        Self {
            file: file.to_string(),
            raises,
            accepted,
        }
    }

    fn verify_revision(git: &Git, base: &str) -> Result<(), TaskFailure> {
        let output = git.run(&[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{base}^{{commit}}"),
        ])?;
        if output.status.success() {
            Ok(())
        } else {
            Err(TaskFailure::unknown_revision(base))
        }
    }

    fn file_at(git: &Git, base: &str, file: &str) -> Result<Option<String>, TaskFailure> {
        let output = git.run(&["show", &format!("{base}:{file}")])?;
        Ok(output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).into_owned()))
    }

    fn trailer_log(git: &Git, base: &str) -> Result<String, TaskFailure> {
        let format = format!("--format=%(trailers:key={},valueonly)", Self::TRAILER);
        let output = git.run(&["log", &format, &format!("{base}..HEAD")])?;
        if !output.status.success() {
            return Err(TaskFailure::tool_failed("git log", &output.stderr));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn accepted_subjects(log: &str) -> BTreeSet<String> {
        log.split_whitespace().map(String::from).collect()
    }

    fn unaccepted(&self) -> impl Iterator<Item = &Raise> {
        self.raises
            .iter()
            .filter(|raise| !self.accepted.contains(&raise.subject))
    }

    fn describe(&self, raise: &Raise) -> String {
        let change = match raise.from {
            Some(before) => format!("rose from {before} to {} in {}", raise.to, self.file),
            None => format!("joined {} with {}", self.file, raise.to),
        };
        format!(
            "{} {change}. Only the project owner may approve that, with a `{}: {}` trailer.",
            raise.subject,
            Self::TRAILER,
            raise.subject
        )
    }
}

impl Check for BaselineRaises {
    fn passes(&self) -> bool {
        self.unaccepted().next().is_none()
    }

    fn summary(&self) -> String {
        format!(
            "{} entries of {} added or raised since the base, {} of them without a `{}:` trailer",
            self.raises.len(),
            self.file,
            self.unaccepted().count(),
            Self::TRAILER
        )
    }

    fn details(&self) -> String {
        self.unaccepted()
            .map(|raise| format!("  {}\n", self.describe(raise)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::BaselineRaises;
    use crate::checks::Check;
    use crate::git::Git;
    use crate::ratchet::Raise;
    use crate::workspace::WorkspaceFixture;

    struct RaiseCase;

    impl RaiseCase {
        const FILE: &str = "xtask/baselines/production_lines.txt";

        fn check(log: &str) -> BaselineRaises {
            let raises = vec![
                Raise {
                    subject: "src/a.rs".to_string(),
                    from: Some(900),
                    to: 901,
                },
                Raise {
                    subject: "src/b.rs".to_string(),
                    from: None,
                    to: 600,
                },
            ];
            BaselineRaises::new(Self::FILE, raises, BaselineRaises::accepted_subjects(log))
        }
    }

    #[test]
    fn a_raise_without_a_trailer_fails() {
        let check = RaiseCase::check("");
        assert!(!check.passes());
        assert_eq!(
            check.details(),
            "  src/a.rs rose from 900 to 901 in xtask/baselines/production_lines.txt. Only the project owner may approve that, with a `Baseline-Accepted: src/a.rs` trailer.\n  src/b.rs joined xtask/baselines/production_lines.txt with 600. Only the project owner may approve that, with a `Baseline-Accepted: src/b.rs` trailer.\n"
        );
    }

    #[test]
    fn a_trailer_accepts_the_raises_it_names() {
        let check = RaiseCase::check("src/a.rs\n\nsrc/b.rs src/c.rs\n");
        assert!(check.passes());
        assert_eq!(
            check.summary(),
            "2 entries of xtask/baselines/production_lines.txt added or raised since the base, 0 of them without a `Baseline-Accepted:` trailer"
        );
    }

    #[test]
    fn a_trailer_for_one_raise_leaves_the_other() {
        let check = RaiseCase::check("src/b.rs\n");
        assert!(check.details().starts_with("  src/a.rs rose"));
        assert!(!check.details().contains("src/b.rs"));
    }

    #[test]
    fn the_baseline_has_no_raise_since_head() {
        let check =
            BaselineRaises::since(WorkspaceFixture::repository_root(), RaiseCase::FILE, "HEAD")
                .expect("the history reads");
        assert!(check.passes());
        assert!(check.raises.is_empty());
    }

    #[test]
    fn an_unknown_base_fails() {
        let failure = BaselineRaises::since(
            WorkspaceFixture::repository_root(),
            RaiseCase::FILE,
            "no-such-revision",
        )
        .err()
        .expect("the base is unknown");
        assert_eq!(
            failure.to_string(),
            "cannot find the revision `no-such-revision`"
        );
    }

    #[test]
    fn a_file_absent_at_the_base_reads_as_absent() {
        let git = Git::new(WorkspaceFixture::repository_root());
        let text =
            BaselineRaises::file_at(&git, "HEAD", "xtask/baselines/absent.txt").expect("git runs");
        assert!(text.is_none());
    }

    #[test]
    fn a_file_present_at_the_base_reads_its_text() {
        let git = Git::new(WorkspaceFixture::repository_root());
        let text = BaselineRaises::file_at(&git, "HEAD", RaiseCase::FILE)
            .expect("git runs")
            .expect("the baseline is committed");
        assert!(text.starts_with("expand/src/"), "{text}");
    }
}

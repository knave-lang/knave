use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::census::Census;
use crate::checks::Check;
use crate::failure::TaskFailure;
use crate::git::Git;
use crate::ratchet::{Comparison, Counts, Measurement, Ratchet};
use crate::revision_tree::RevisionTree;
use crate::rise_approvals::RiseApprovals;
use crate::verdict::Verdict;

pub struct RatchetGate {
    root: PathBuf,
    base: String,
}

impl RatchetGate {
    pub fn new(root: &Path, base: &str) -> Self {
        Self {
            root: root.to_path_buf(),
            base: base.to_string(),
        }
    }

    pub fn run(&self, censuses: &[&dyn Census]) -> Result<ExitCode, TaskFailure> {
        let checks = self.checks(censuses)?;
        let checks: Vec<&dyn Check> = checks.iter().map(Box::as_ref).collect();
        Ok(Verdict::announce(&checks))
    }

    pub fn checks(&self, censuses: &[&dyn Census]) -> Result<Vec<Box<dyn Check>>, TaskFailure> {
        let comparison = self.comparison()?;
        let mut checks = Vec::new();
        for census in censuses {
            checks.extend(comparison.ratchets(*census)?);
        }
        Ok(checks)
    }

    fn comparison(&self) -> Result<BaseComparison<'_>, TaskFailure> {
        let git = Git::new(&self.root);
        let merge_base = git.merge_base(&self.base)?;
        Ok(BaseComparison {
            head_root: &self.root,
            approvals: RiseApprovals::since(&git, &merge_base)?,
            tree: RevisionTree::extract(&self.root, &merge_base)?,
        })
    }
}

struct BaseComparison<'a> {
    head_root: &'a Path,
    tree: RevisionTree,
    approvals: RiseApprovals,
}

impl BaseComparison<'_> {
    fn ratchets(&self, census: &dyn Census) -> Result<Vec<Box<dyn Check>>, TaskFailure> {
        let head = census.measure(self.head_root)?;
        let base = census.measure(self.tree.root())?;
        Ok(Self::compare(head, base)
            .into_iter()
            .map(|comparison| Box::new(Ratchet::new(comparison, &self.approvals)) as Box<dyn Check>)
            .collect())
    }

    fn compare(head: Vec<Measurement>, base: Vec<Measurement>) -> Vec<Comparison> {
        let mut base_counts: BTreeMap<&str, Counts> = base
            .into_iter()
            .map(|measurement| (measurement.limit.name, measurement.counts))
            .collect();
        head.into_iter()
            .map(|measurement| Comparison {
                base: base_counts
                    .remove(measurement.limit.name)
                    .unwrap_or_default(),
                limit: measurement.limit,
                head: measurement.counts,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{BaseComparison, RatchetGate};
    use crate::census::Census;
    use crate::failure::TaskFailure;
    use crate::production_lines::ProductionLineCensus;
    use crate::ratchet::{Counts, Limit, Measurement};
    use crate::temporary_repository::TemporaryRepository;

    struct CountsFile;

    impl CountsFile {
        const PATH: &str = "counts.txt";
    }

    impl Census for CountsFile {
        fn measure(&self, root: &Path) -> Result<Vec<Measurement>, TaskFailure> {
            let text = fs::read_to_string(root.join(Self::PATH)).unwrap_or_default();
            let counts = text
                .lines()
                .filter_map(|line| line.split_once(' '))
                .map(|(subject, count)| (subject.to_string(), count.parse().expect("a count")))
                .collect();
            Ok(vec![Measurement {
                limit: ProductionLineCensus::LIMIT,
                counts,
            }])
        }
    }

    struct Branch {
        repository: TemporaryRepository,
    }

    impl Branch {
        const BASE: &str = "base";

        fn forked_with(name: &str, counts: &str) -> Self {
            let repository = TemporaryRepository::initialized(name);
            repository.write(CountsFile::PATH, counts);
            repository.commit_all();
            repository.git(&["branch", Self::BASE]);
            Self { repository }
        }

        fn commit(&self, counts: &str, message: &str) {
            self.repository.write(CountsFile::PATH, counts);
            self.repository.commit_all_with_message(message);
        }

        fn advance_base(&self, counts: &str) {
            self.repository.git(&["checkout", "--quiet", Self::BASE]);
            self.commit(counts, "base");
            self.repository.git(&["checkout", "--quiet", "-"]);
        }

        fn details(&self) -> Vec<String> {
            RatchetGate::new(self.repository.root(), Self::BASE)
                .checks(&[&CountsFile])
                .expect("the gate measures")
                .iter()
                .map(|check| check.details())
                .collect()
        }
    }

    #[test]
    fn a_rise_since_the_merge_base_is_refused() {
        let branch = Branch::forked_with("gate-rise", "src/a.rs 900\n");
        branch.commit("src/a.rs 901\n", "grow");
        assert_eq!(
            branch.details(),
            [
                "  src/a.rs has 901 production lines, over the limit of 500, and 900 at the base. Split it by responsibility. If the project owner approves the rise, add this trailer to a commit of the branch:\n    Baseline-Accepted: production_lines src/a.rs\n"
            ]
        );
    }

    #[test]
    fn a_trailer_in_a_branch_commit_accepts_the_rise() {
        let branch = Branch::forked_with("gate-trailer", "src/a.rs 900\n");
        branch.commit(
            "src/a.rs 901\n",
            "grow\n\nBaseline-Accepted: production_lines src/a.rs",
        );
        assert_eq!(branch.details(), [""]);
    }

    #[test]
    fn the_working_tree_is_measured_against_the_merge_base_not_the_base_tip() {
        let branch = Branch::forked_with("gate-merge-base", "src/a.rs 900\n");
        branch.advance_base("src/a.rs 950\n");
        branch.commit("src/a.rs 920\n", "grow");
        assert_eq!(branch.details().len(), 1);
        assert!(branch.details()[0].contains("and 900 at the base"));
    }

    #[test]
    fn a_fall_and_a_deletion_pass() {
        let branch = Branch::forked_with("gate-fall", "src/a.rs 900\nsrc/b.rs 700\n");
        branch.commit("src/a.rs 899\n", "shrink");
        assert_eq!(branch.details(), [""]);
    }

    #[test]
    fn head_and_base_pair_by_limit_name_and_a_limit_missing_at_the_base_counts_nothing() {
        let measurement = |name: &'static str, count: usize| Measurement {
            limit: Limit {
                name,
                ..ProductionLineCensus::LIMIT
            },
            counts: Counts::from([("src/a.rs".to_string(), count)]),
        };
        let comparisons = BaseComparison::compare(
            vec![measurement("first", 1), measurement("second", 2)],
            vec![measurement("second", 20)],
        );
        let paired: Vec<(&str, usize, Option<usize>)> = comparisons
            .iter()
            .map(|comparison| {
                (
                    comparison.limit.name,
                    comparison.head["src/a.rs"],
                    comparison.base.get("src/a.rs").copied(),
                )
            })
            .collect();
        assert_eq!(paired, [("first", 1, None), ("second", 2, Some(20))]);
    }

    #[test]
    fn an_unknown_base_fails() {
        let repository = TemporaryRepository::initialized("gate-unknown");
        repository.write(CountsFile::PATH, "");
        repository.commit_all();
        let failure = RatchetGate::new(repository.root(), "no-such-revision")
            .checks(&[&CountsFile])
            .err()
            .expect("the base is unknown");
        assert_eq!(
            failure.to_string(),
            "cannot find the revision `no-such-revision`"
        );
    }
}

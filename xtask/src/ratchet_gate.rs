use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::baseline_raises::BaselineRaises;
use crate::checks::Check;
use crate::failure::TaskFailure;
use crate::ratchet::Ratchet;
use crate::verdict::Verdict;

pub struct RatchetGate {
    root: PathBuf,
    base: Option<String>,
}

impl RatchetGate {
    pub fn new(root: &Path, base: Option<&str>) -> Self {
        Self {
            root: root.to_path_buf(),
            base: base.map(String::from),
        }
    }

    pub fn run(&self, ratchets: Vec<Ratchet>) -> Result<ExitCode, TaskFailure> {
        let checks = self.checks(ratchets)?;
        let checks: Vec<&dyn Check> = checks.iter().map(Box::as_ref).collect();
        Ok(Verdict::announce(&checks))
    }

    fn checks(&self, ratchets: Vec<Ratchet>) -> Result<Vec<Box<dyn Check>>, TaskFailure> {
        let raises = self.raises(&ratchets)?;
        let ratchets = ratchets
            .into_iter()
            .map(|ratchet| Box::new(ratchet) as Box<dyn Check>);
        let raises = raises
            .into_iter()
            .map(|raise| Box::new(raise) as Box<dyn Check>);
        Ok(ratchets.chain(raises).collect())
    }

    fn raises(&self, ratchets: &[Ratchet]) -> Result<Vec<BaselineRaises>, TaskFailure> {
        let Some(base) = &self.base else {
            return Ok(Vec::new());
        };
        ratchets
            .iter()
            .map(|ratchet| BaselineRaises::since(&self.root, ratchet.baseline_file(), base))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::RatchetGate;
    use crate::production_lines::ProductionLineCensus;
    use crate::ratchet::{Counts, Ratchet};
    use crate::workspace::WorkspaceFixture;

    struct Gate;

    impl Gate {
        fn summaries(base: Option<&str>) -> Vec<String> {
            let root = WorkspaceFixture::repository_root();
            let ratchet = Ratchet::read(root, ProductionLineCensus::LIMIT, Counts::new())
                .expect("the baseline reads");
            RatchetGate::new(root, base)
                .checks(vec![ratchet])
                .expect("the history reads")
                .iter()
                .map(|check| check.summary())
                .collect()
        }
    }

    #[test]
    fn without_a_base_only_the_ratchets_run() {
        assert_eq!(Gate::summaries(None).len(), 1);
    }

    #[test]
    fn a_base_adds_a_raise_check_for_each_ratchet_baseline() {
        let summaries = Gate::summaries(Some("HEAD"));
        assert_eq!(summaries.len(), 2);
        assert!(
            summaries[1]
                .contains("of xtask/baselines/production_lines.txt added or raised since the base"),
            "{}",
            summaries[1]
        );
    }
}

use std::path::Path;
use std::process::ExitCode;

use clap::Args;

use crate::changed_functions::ChangedFunctions;
use crate::changed_lines::ChangedLines;
use crate::checks::Check;
use crate::crap_forecast::CrapForecast;
use crate::design::DesignRules;
use crate::failure::TaskFailure;
use crate::git::Git;
use crate::narrow_coverage::NarrowCoverage;
use crate::production_lines::ProductionLineCensus;
use crate::production_sources::ProductionSources;
use crate::ratchet_gate::RatchetGate;
use crate::source_format::SourceFormat;
use crate::test_selection::TestRun;
use crate::verdict::Verdict;

#[derive(Args)]
pub struct PrepushTask {
    #[arg(
        long,
        default_value = "origin/main",
        help = "Check the changes since this branch forked from this revision"
    )]
    base: String,
    #[arg(
        long,
        allow_hyphen_values = true,
        help = "Also run these `cargo test` arguments under coverage, such as '--test stages emit_fixtures'"
    )]
    also: Vec<String>,
}

impl PrepushTask {
    const AFTER_A_REBASE: &str = "After each rebase, run `cargo xtask prepush` again: git's baseline merge driver adds both branches' changes to each entry, and file-sizes and design-rules catch an entry whose sum is off.";

    pub fn run(&self, root: &Path) -> ExitCode {
        let verdict = self
            .attempt(root)
            .unwrap_or_else(|failure| failure.report());
        println!("{}", Self::AFTER_A_REBASE);
        verdict
    }

    fn attempt(&self, root: &Path) -> Result<ExitCode, TaskFailure> {
        let fork_point = self.fork_point(root)?;
        let changed = ChangedLines::since(root, &fork_point)?;
        let mut checks: Vec<Box<dyn Check>> = vec![
            Box::new(self.crap_forecast(root, &changed)?),
            Box::new(SourceFormat::check(root, &changed)?),
        ];
        checks.extend(Self::ratchet_checks(root, &fork_point)?);
        let checks: Vec<&dyn Check> = checks.iter().map(Box::as_ref).collect();
        Ok(Verdict::announce(&checks))
    }

    fn fork_point(&self, root: &Path) -> Result<String, TaskFailure> {
        let output = Git::new(root).run(&["merge-base", &self.base, "HEAD"])?;
        if !output.status.success() {
            return Err(TaskFailure::unknown_revision(&self.base));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn crap_forecast(
        &self,
        root: &Path,
        changed: &ChangedLines,
    ) -> Result<CrapForecast, TaskFailure> {
        let functions = ChangedFunctions::find(root, changed)?;
        let also: Vec<TestRun> = self
            .also
            .iter()
            .map(|arguments| TestRun::parse(arguments))
            .collect();
        let report = NarrowCoverage::new(root).measure(&functions.test_runs(&also))?;
        Ok(functions.forecast(&report))
    }

    fn ratchet_checks(root: &Path, fork_point: &str) -> Result<Vec<Box<dyn Check>>, TaskFailure> {
        let sources = ProductionSources::new(root);
        let mut ratchets = DesignRules::ratchets(root, &sources.parsed()?)?;
        ratchets.push(ProductionLineCensus::ratchet(root, &sources.files()?)?);
        RatchetGate::new(root, Some(fork_point)).checks(ratchets)
    }
}

#[cfg(test)]
mod tests {
    use super::PrepushTask;
    use crate::workspace::WorkspaceFixture;

    #[test]
    fn an_unknown_base_fails_before_any_check() {
        let task = PrepushTask {
            base: "no-such-revision".to_string(),
            also: Vec::new(),
        };
        let failure = match task.fork_point(WorkspaceFixture::repository_root()) {
            Ok(fork_point) => panic!("found a fork point {fork_point}"),
            Err(failure) => failure.to_string(),
        };
        assert_eq!(failure, "cannot find the revision `no-such-revision`");
    }
}

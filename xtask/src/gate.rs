use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::baseline_raises::BaselineRaises;
use crate::checks::{BadgeFreshness, Check, CleanEmittedRust, CoverageFloor, CrapLimit};
use crate::crap::ScoredFunction;
use crate::design::DesignRules;
use crate::emitted_lints::{EmittedLints, LintedProgram};
use crate::failure::TaskFailure;
use crate::functions::{SourceFunction, SourceTree};
use crate::instrumented_tests::InstrumentedTests;
use crate::language::LanguageSource;
use crate::lcov::LcovReport;
use crate::parsed_source::ParsedSource;
use crate::production_lines::ProductionLineCensus;
use crate::ratchet::{Limit, Ratchet, Unit};
use crate::source_file::SourceFile;
use crate::workspace::CargoWorkspace;

pub struct QualityGate {
    root: PathBuf,
}

struct CoverageEvidence {
    report: LcovReport,
    functions: Vec<SourceFunction>,
    readme: String,
}

impl QualityGate {
    const MINIMUM_LINE_COVERAGE: f64 = 99.0;
    const MAXIMUM_CRAP: f64 = 8.0;
    const MAXIMUM_PRODUCTION_LINES: usize = 500;
    const PRODUCTION_LINES: Limit = Limit {
        baseline: "xtask/baselines/production_lines.txt",
        maximum: Self::MAXIMUM_PRODUCTION_LINES,
        subjects: "Rust files",
        unit: Unit::new("production line", "production lines"),
        advice: "Split it by responsibility.",
    };
    const README: &str = "README.md";
    const EXTENSION_CONSTANT: &str = "EXTENSION";

    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub fn run(&self) -> Result<ExitCode, TaskFailure> {
        let (evidence, linted) = self.measure_everything()?;
        Ok(Self::judge(evidence, &[&CleanEmittedRust::new(linted)]))
    }

    pub fn run_on_shards(&self) -> Result<ExitCode, TaskFailure> {
        let report = InstrumentedTests::new(&self.root).merge_shards()?;
        Ok(Self::judge(self.evidence(report)?, &[]))
    }

    pub fn run_emitted_lints(&self) -> Result<ExitCode, TaskFailure> {
        let linted = self.lint_emitted_programs()?;
        Ok(Self::conclude(&[&CleanEmittedRust::new(linted)]))
    }

    pub fn run_file_sizes(&self, base: Option<&str>) -> Result<ExitCode, TaskFailure> {
        let checks = self.file_size_checks(base)?;
        let checks: Vec<&dyn Check> = checks.iter().map(Box::as_ref).collect();
        Ok(Self::conclude(&checks))
    }

    fn file_size_checks(&self, base: Option<&str>) -> Result<Vec<Box<dyn Check>>, TaskFailure> {
        let measured = ProductionLineCensus::measure(&self.production_sources()?);
        let ratchet = Ratchet::read(&self.root, Self::PRODUCTION_LINES, measured)?;
        let mut checks: Vec<Box<dyn Check>> = vec![Box::new(ratchet)];
        if let Some(base) = base {
            let baseline = Self::PRODUCTION_LINES.baseline;
            checks.push(Box::new(BaselineRaises::since(&self.root, baseline, base)?));
        }
        Ok(checks)
    }

    pub fn run_design_rules(&self, base: Option<&str>) -> Result<ExitCode, TaskFailure> {
        let checks = self.design_rule_checks(base)?;
        let checks: Vec<&dyn Check> = checks.iter().map(Box::as_ref).collect();
        Ok(Self::conclude(&checks))
    }

    fn design_rule_checks(&self, base: Option<&str>) -> Result<Vec<Box<dyn Check>>, TaskFailure> {
        let sources = self.parsed_production_sources()?;
        let mut checks: Vec<Box<dyn Check>> = DesignRules::ratchets(&self.root, &sources)?
            .into_iter()
            .map(|ratchet| Box::new(ratchet) as Box<dyn Check>)
            .collect();
        if let Some(base) = base {
            for baseline in DesignRules::baselines() {
                checks.push(Box::new(BaselineRaises::since(&self.root, baseline, base)?));
            }
        }
        Ok(checks)
    }

    fn parsed_production_sources(&self) -> Result<Vec<ParsedSource>, TaskFailure> {
        let directories = self.source_directories()?;
        let crate_directories: Vec<String> = directories
            .iter()
            .map(|directory| {
                directory
                    .strip_prefix(&self.root)
                    .unwrap_or(directory)
                    .display()
                    .to_string()
            })
            .collect();
        let files = SourceTree::new(&self.root, directories).files()?;
        Ok(ParsedSource::all_production(files, &crate_directories))
    }

    fn production_sources(&self) -> Result<Vec<SourceFile>, TaskFailure> {
        SourceTree::new(&self.root, self.source_directories()?).files()
    }

    fn source_directories(&self) -> Result<BTreeSet<PathBuf>, TaskFailure> {
        Ok(CargoWorkspace::read(&self.root)?.source_directories_with_tooling())
    }

    fn measure_everything(&self) -> Result<(CoverageEvidence, Vec<LintedProgram>), TaskFailure> {
        let report = InstrumentedTests::new(&self.root).run()?;
        let evidence = self.evidence(report)?;
        let linted = self.lint_emitted_programs()?;
        Ok((evidence, linted))
    }

    fn lint_emitted_programs(&self) -> Result<Vec<LintedProgram>, TaskFailure> {
        let workspace = CargoWorkspace::read(&self.root)?;
        let extension = LanguageSource::find(&workspace)?.constant(Self::EXTENSION_CONSTANT)?;
        EmittedLints::new(&self.root, &extension).run()
    }

    fn evidence(&self, report: LcovReport) -> Result<CoverageEvidence, TaskFailure> {
        let directories = CargoWorkspace::read(&self.root)?.source_directories();
        let functions = SourceTree::new(&self.root, directories).functions()?;
        let readme_path = self.root.join(Self::README);
        let readme = fs::read_to_string(&readme_path)
            .map_err(|error| TaskFailure::unreadable(&readme_path, error))?;
        Ok(CoverageEvidence {
            report,
            functions,
            readme,
        })
    }

    fn judge(evidence: CoverageEvidence, further_checks: &[&dyn Check]) -> ExitCode {
        let coverage = evidence.report.total();
        let scored = Self::score(evidence.functions, &evidence.report);
        let floor = CoverageFloor::new(Self::MINIMUM_LINE_COVERAGE, &evidence.report);
        let crap = CrapLimit::new(Self::MAXIMUM_CRAP, scored);
        let badge = BadgeFreshness::new(&evidence.readme, coverage);
        let coverage_checks: [&dyn Check; 3] = [&floor, &crap, &badge];
        Self::conclude(&[&coverage_checks[..], further_checks].concat())
    }

    fn conclude(checks: &[&dyn Check]) -> ExitCode {
        let failures = checks
            .iter()
            .filter(|check| !Self::announce(**check))
            .count();
        if failures == 0 {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }

    fn score(functions: Vec<SourceFunction>, report: &LcovReport) -> Vec<ScoredFunction> {
        functions
            .into_iter()
            .map(|function| {
                let coverage = report.within(&function.file, function.lines);
                ScoredFunction::new(function, coverage)
            })
            .collect()
    }

    fn announce(check: &dyn Check) -> bool {
        let passes = check.passes();
        if passes {
            println!("ok: {}", check.summary());
        } else {
            println!("error: {}\n{}", check.summary(), check.details());
        }
        passes
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::process::ExitCode;

    use super::{CoverageEvidence, QualityGate};
    use crate::badge::CoverageBadge;
    use crate::checks::Check;
    use crate::lcov::LcovReport;
    use crate::workspace::WorkspaceFixture;

    struct FixedCheck {
        passes: bool,
    }

    impl Check for FixedCheck {
        fn passes(&self) -> bool {
            self.passes
        }

        fn summary(&self) -> String {
            "a fixed check".to_string()
        }

        fn details(&self) -> String {
            String::new()
        }
    }

    impl CoverageEvidence {
        fn fully_covered() -> Self {
            let report = LcovReport::parse("SF:/work/src/a.rs\nDA:1,1\n", Path::new("/work"));
            let readme = CoverageBadge::expected(report.total());
            Self {
                report,
                functions: Vec::new(),
                readme,
            }
        }
    }

    #[test]
    fn file_sizes_also_check_raises_against_a_base() {
        let gate = QualityGate::new(WorkspaceFixture::repository_root());
        let summaries = |base| -> Vec<String> {
            gate.file_size_checks(base)
                .expect("the repository reads")
                .iter()
                .map(|check| check.summary())
                .collect()
        };
        assert_eq!(summaries(None).len(), 1);
        let with_base = summaries(Some("HEAD"));
        assert_eq!(with_base.len(), 2);
        assert!(
            with_base[1].contains("added or raised since the base"),
            "{}",
            with_base[1]
        );
    }

    #[test]
    fn design_rules_check_every_baseline_against_a_base() {
        let gate = QualityGate::new(WorkspaceFixture::repository_root());
        let summaries = |base| -> Vec<String> {
            gate.design_rule_checks(base)
                .expect("the repository reads")
                .iter()
                .map(|check| check.summary())
                .collect()
        };
        assert_eq!(summaries(None).len(), 9);
        let raises: Vec<String> = summaries(Some("HEAD"))
            .into_iter()
            .filter(|summary| summary.contains("added or raised since the base"))
            .collect();
        assert_eq!(raises.len(), 9, "{raises:?}");
    }

    #[test]
    fn the_production_line_limit_is_500() {
        assert_eq!(QualityGate::PRODUCTION_LINES.maximum, 500);
    }

    #[test]
    fn file_sizes_measure_each_crate_and_the_tooling() {
        let gate = QualityGate::new(WorkspaceFixture::repository_root());
        let sources = gate.production_sources().expect("the repository reads");
        for directory in ["src/", "expand/src/", "xtask/src/"] {
            assert!(
                sources
                    .iter()
                    .any(|source| source.relative().starts_with(directory)),
                "no source under {directory}"
            );
        }
    }

    #[test]
    fn covered_evidence_passes_with_passing_further_checks() {
        let passing = FixedCheck { passes: true };
        let verdict = QualityGate::judge(CoverageEvidence::fully_covered(), &[&passing]);
        assert_eq!(verdict, ExitCode::SUCCESS);
    }

    #[test]
    fn a_failing_further_check_fails_the_gate() {
        let failing = FixedCheck { passes: false };
        let verdict = QualityGate::judge(CoverageEvidence::fully_covered(), &[&failing]);
        assert_eq!(verdict, ExitCode::FAILURE);
    }
}

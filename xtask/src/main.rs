mod badge;
mod baseline_raises;
mod bench;
mod cargo;
mod checks;
mod complexity;
mod coverage;
mod crap;
mod design;
mod emitted_lints;
mod failure;
mod function_owner;
mod functions;
mod gate;
mod git;
mod instrumented_tests;
mod language;
mod lcov;
mod parsed_source;
mod production_lines;
mod production_sources;
mod ratchet;
mod ratchet_gate;
mod release;
mod source_file;
mod test_code;
mod test_modules;
mod test_shard;
mod verdict;
mod workspace;

use std::path::Path;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

use bench::BenchTask;
use design::DesignRules;
use failure::TaskFailure;
use gate::QualityGate;
use instrumented_tests::InstrumentedTests;
use production_lines::ProductionLineCensus;
use production_sources::ProductionSources;
use ratchet_gate::RatchetGate;
use release::{NextReleaseTask, ReleaseNotesTask, SetVersionTask};
use test_shard::TestShard;

#[derive(Parser)]
#[command(about = "Development tasks for this repository")]
struct Tasks {
    #[command(subcommand)]
    task: Task,
}

#[derive(Subcommand)]
enum Task {
    #[command(
        about = "Run the tests with coverage, then check the coverage floor, the CRAP limit, the README badge and the emitted Rust's lints"
    )]
    Coverage(CoverageTask),
    #[command(
        about = "Check that no Rust file outgrows the production line limit or its baseline entry"
    )]
    FileSizes(RatchetTask),
    #[command(
        about = "Check the design rules of the Rust code: type, function and module limits, each with its baseline"
    )]
    DesignRules(RatchetTask),
    #[command(
        subcommand,
        about = "Measure the compiler's front end under callgrind, or compare two measurements"
    )]
    Bench(BenchTask),
    #[command(
        about = "Print the tag that follows the latest release, bumped by the pull requests' labels"
    )]
    NextRelease(NextReleaseTask),
    #[command(about = "Write a version into the crate's manifest and its lock file entry")]
    SetVersion(SetVersionTask),
    #[command(
        about = "Print the release notes: the pull requests with a release label first, then the rest"
    )]
    ReleaseNotes(ReleaseNotesTask),
}

#[derive(Args)]
struct RatchetTask {
    #[arg(
        long,
        help = "Also refuse a baseline entry added or raised since this revision, unless a `Baseline-Accepted:` trailer names its baseline and subject"
    )]
    base: Option<String>,
}

#[derive(Args)]
struct CoverageTask {
    #[command(subcommand)]
    part: Option<CoveragePart>,
}

#[derive(Subcommand)]
enum CoveragePart {
    #[command(about = "Run one shard of the tests with coverage and write its report")]
    Shard { shard: TestShard },
    #[command(about = "Check the floor, the CRAP limit and the badge against the shards' reports")]
    Gate,
    #[command(about = "Check the emitted Rust's lints")]
    EmittedLints,
}

impl RatchetTask {
    fn check_file_sizes(&self, workspace_root: &Path) -> Result<ExitCode, TaskFailure> {
        let files = ProductionSources::new(workspace_root).files()?;
        let ratchet = ProductionLineCensus::ratchet(workspace_root, &files)?;
        self.gate(workspace_root).run(vec![ratchet])
    }

    fn check_design_rules(&self, workspace_root: &Path) -> Result<ExitCode, TaskFailure> {
        let sources = ProductionSources::new(workspace_root).parsed()?;
        let ratchets = DesignRules::ratchets(workspace_root, &sources)?;
        self.gate(workspace_root).run(ratchets)
    }

    fn gate(&self, workspace_root: &Path) -> RatchetGate {
        RatchetGate::new(workspace_root, self.base.as_deref())
    }
}

impl CoverageTask {
    fn run(self, workspace_root: &Path) -> ExitCode {
        self.attempt(workspace_root)
            .unwrap_or_else(|failure| failure.report())
    }

    fn attempt(self, workspace_root: &Path) -> Result<ExitCode, TaskFailure> {
        let gate = QualityGate::new(workspace_root);
        match self.part {
            None => gate.run(),
            Some(CoveragePart::Shard { shard }) => Self::run_shard(workspace_root, shard),
            Some(CoveragePart::Gate) => gate.run_on_shards(),
            Some(CoveragePart::EmittedLints) => gate.run_emitted_lints(),
        }
    }

    fn run_shard(workspace_root: &Path, shard: TestShard) -> Result<ExitCode, TaskFailure> {
        InstrumentedTests::new(workspace_root).run_shard(shard)?;
        Ok(ExitCode::SUCCESS)
    }
}

impl Task {
    fn run(self) -> ExitCode {
        let manifest_directory = Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_directory.parent().unwrap_or(manifest_directory);
        match self {
            Task::Coverage(coverage) => coverage.run(workspace_root),
            Task::FileSizes(sizes) => sizes
                .check_file_sizes(workspace_root)
                .unwrap_or_else(|failure| failure.report()),
            Task::DesignRules(rules) => rules
                .check_design_rules(workspace_root)
                .unwrap_or_else(|failure| failure.report()),
            Task::Bench(task) => task.run(workspace_root),
            Task::NextRelease(release) => release.run(),
            Task::SetVersion(version) => version.run(workspace_root),
            Task::ReleaseNotes(notes) => notes.run(),
        }
    }
}

fn main() -> ExitCode {
    Tasks::parse().task.run()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use clap::Parser;

    use super::{CoveragePart, CoverageTask, Task, Tasks};
    use crate::test_shard::TestShard;

    struct CommandLine;

    impl CommandLine {
        const WORKSPACE_THAT_IS_A_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");

        fn coverage(arguments: &[&str]) -> CoverageTask {
            let command_line = [&["xtask", "coverage"], arguments].concat();
            match Tasks::try_parse_from(command_line)
                .expect("the command line parses")
                .task
            {
                Task::Coverage(coverage) => coverage,
                _ => panic!("the command line names another task"),
            }
        }

        fn coverage_part(arguments: &[&str]) -> Option<CoveragePart> {
            Self::coverage(arguments).part
        }

        fn failure_in_file_workspace(arguments: &[&str]) -> String {
            match Self::coverage(arguments).attempt(Path::new(Self::WORKSPACE_THAT_IS_A_FILE)) {
                Ok(_) => panic!("the task finished in a workspace that is a file"),
                Err(failure) => failure.to_string(),
            }
        }
    }

    #[test]
    fn file_sizes_takes_an_optional_base() {
        let base = |arguments: &[&str]| match Tasks::try_parse_from(arguments)
            .expect("the command line parses")
            .task
        {
            Task::FileSizes(sizes) => sizes.base,
            _ => panic!("the command line names another task"),
        };
        assert_eq!(base(&["xtask", "file-sizes"]), None);
        assert_eq!(
            base(&["xtask", "file-sizes", "--base", "HEAD^1"]),
            Some("HEAD^1".to_string())
        );
    }

    #[test]
    fn design_rules_take_an_optional_base() {
        let base = |arguments: &[&str]| match Tasks::try_parse_from(arguments)
            .expect("the command line parses")
            .task
        {
            Task::DesignRules(rules) => rules.base,
            _ => panic!("the command line names another task"),
        };
        assert_eq!(base(&["xtask", "design-rules"]), None);
        assert_eq!(
            base(&["xtask", "design-rules", "--base", "HEAD^1"]),
            Some("HEAD^1".to_string())
        );
    }

    #[test]
    fn coverage_alone_runs_everything() {
        assert!(CommandLine::coverage_part(&[]).is_none());
    }

    #[test]
    fn coverage_alone_writes_the_single_report_first() {
        let failure = CommandLine::failure_in_file_workspace(&[]);
        assert!(failure.contains("target/coverage`"), "{failure}");
    }

    #[test]
    fn coverage_gate_reads_the_shard_reports_first() {
        let failure = CommandLine::failure_in_file_workspace(&["gate"]);
        assert!(failure.contains("shards/"), "{failure}");
    }

    #[test]
    fn coverage_shard_names_its_shard() {
        assert!(matches!(
            CommandLine::coverage_part(&["shard", "rest"]),
            Some(CoveragePart::Shard {
                shard: TestShard::Rest
            })
        ));
    }

    #[test]
    fn coverage_gate_judges_the_shards() {
        assert!(matches!(
            CommandLine::coverage_part(&["gate"]),
            Some(CoveragePart::Gate)
        ));
    }

    #[test]
    fn coverage_emitted_lints_lints_alone() {
        assert!(matches!(
            CommandLine::coverage_part(&["emitted-lints"]),
            Some(CoveragePart::EmittedLints)
        ));
    }
}

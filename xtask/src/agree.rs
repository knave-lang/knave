use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

use clap::Args;
use knave_agree::{
    AgreementRun, FindingList, FindingsDirectory, FormReducer, Harness, KnaveCompiler,
    KnownClasses, ProgramGenerator, ProgramSource, ReducedFindings, ReductionPlan, RunResults,
    Seed, SeedRange,
};

use crate::failure::TaskFailure;

#[derive(Args)]
pub struct AgreeTask {
    #[arg(
        long,
        default_value = "0..20",
        help = "The seeds of the generated programs, as `<first>..<end>`"
    )]
    seeds: SeedRange,
    #[arg(long, default_value_t = AgreeTask::STATEMENTS, help = "The statements of each program's `main`")]
    statements: usize,
    #[arg(
        long = "oracle",
        value_name = "NAME",
        help = "Judge with this oracle only; repeat to name several [default: every oracle]"
    )]
    oracles: Vec<String>,
    #[arg(
        long,
        value_name = "FILE",
        help = "The compiler to test [default: target/debug/knave]"
    )]
    knave: Option<PathBuf>,
    #[arg(
        long,
        value_name = "FILE",
        help = "Pass when the findings are exactly this file's lines for the seeds that ran [default: xtask/agree/pull-request-findings.txt]"
    )]
    expected: Option<PathBuf>,
    #[arg(
        long,
        help = "How many programs run at once [default: the number of cores]"
    )]
    jobs: Option<usize>,
    #[arg(
        long,
        value_name = "SEED",
        help = "Print the program of this seed and run nothing"
    )]
    print: Option<u64>,
    #[arg(
        long,
        value_name = "COUNT",
        default_value_t = 0,
        help = "Shrink up to this many findings, those outside the known classes first, and write each as `<seed>.reduced.knv`"
    )]
    reduce: usize,
    #[arg(
        long,
        value_name = "FILE",
        help = "Write the reduced findings outside the known classes to this Markdown file, if there are any"
    )]
    report: Option<PathBuf>,
}

impl AgreeTask {
    const STATEMENTS: usize = 14;
    const STEP_LIMIT: Duration = Duration::from_secs(60);
    const REDUCTION_ATTEMPTS: usize = 400;
    const DEFAULT_COMPILER: &str = "target/debug/knave";
    const FINDINGS_DIRECTORY: &str = "target/agree";
    const RECORDED_FINDINGS: &str = "xtask/agree/pull-request-findings.txt";

    pub fn run(self, workspace_root: &Path) -> ExitCode {
        let outcome = match self.print {
            Some(seed) => Ok(self.print_program(Seed::from(seed))),
            None => self.attempt(workspace_root),
        };
        outcome.unwrap_or_else(|failure| failure.report())
    }

    fn print_program(&self, seed: Seed) -> ExitCode {
        print!("{}", self.generator().program(seed).entry_text());
        ExitCode::SUCCESS
    }

    fn attempt(&self, workspace_root: &Path) -> Result<ExitCode, TaskFailure> {
        let harness = Harness::new(
            KnaveCompiler::at(self.compiler(workspace_root)?),
            self.jobs(),
        )
        .keeping_oracles(&self.oracles)
        .map_err(TaskFailure::new)?;
        let started = Instant::now();
        let results =
            AgreementRun::new(&harness, Self::STEP_LIMIT).run(self.generator().programs());
        println!("{}", results.tally);
        println!(
            "{} seconds, {} jobs",
            started.elapsed().as_secs_f64(),
            harness.jobs()
        );
        let directory = FindingsDirectory::at(workspace_root.join(Self::FINDINGS_DIRECTORY));
        let found = Self::write_findings(&directory, &results)?;
        self.reduce(&harness, &results, &directory)?;
        let expected = self.expected_findings(workspace_root)?.within(&self.seeds);
        let changes = found.changes_from(&expected);
        eprint!("{changes}");
        Ok(if changes.is_empty() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        })
    }

    fn generator(&self) -> ProgramGenerator {
        ProgramGenerator::new(self.seeds.clone(), self.statements)
    }

    fn compiler(&self, workspace_root: &Path) -> Result<PathBuf, TaskFailure> {
        let compiler = self
            .knave
            .clone()
            .unwrap_or_else(|| workspace_root.join(Self::DEFAULT_COMPILER));
        if compiler.is_file() {
            Ok(compiler)
        } else {
            Err(TaskFailure::new(format!(
                "no compiler at `{}`: run `cargo build` first, or name one with `--knave`",
                compiler.display()
            )))
        }
    }

    fn jobs(&self) -> usize {
        self.jobs
            .unwrap_or_else(|| thread::available_parallelism().map_or(1, NonZeroUsize::get))
    }

    fn reduce(
        &self,
        harness: &Harness,
        results: &RunResults,
        directory: &FindingsDirectory,
    ) -> Result<(), TaskFailure> {
        let plan = ReductionPlan::new(
            FormReducer::new(Self::STEP_LIMIT, Self::REDUCTION_ATTEMPTS),
            &KnownClasses::BEFORE_THE_LOAN_RULE,
            self.reduce,
        );
        let reduced = ReducedFindings::of(harness, &results.findings, &plan);
        reduced
            .record(directory)
            .map_err(|error| TaskFailure::unwritable(directory.path(), error))?;
        match (&self.report, reduced.report_outside_known_classes()) {
            (Some(path), Some(report)) => {
                fs::write(path, report).map_err(|error| TaskFailure::unwritable(path, error))
            }
            _ => Ok(()),
        }
    }

    fn write_findings(
        directory: &FindingsDirectory,
        results: &RunResults,
    ) -> Result<FindingList, TaskFailure> {
        let found = directory
            .record(&results.findings)
            .map_err(|error| TaskFailure::unwritable(directory.path(), error))?;
        print!("{}", found.text());
        Ok(found)
    }

    fn expected_findings(&self, workspace_root: &Path) -> Result<FindingList, TaskFailure> {
        let path = self
            .expected
            .clone()
            .unwrap_or_else(|| workspace_root.join(Self::RECORDED_FINDINGS));
        FindingList::read(&path).map_err(|error| TaskFailure::unreadable(&path, error))
    }
}

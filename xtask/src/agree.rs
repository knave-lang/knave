use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

use clap::Args;
use knave_agree::{
    AgreementRun, FindingList, Harness, KnaveCompiler, ProgramGenerator, ProgramSource, RunResults,
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
        help = "Pass when the findings are exactly this file's lines, not when there are none"
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
}

impl AgreeTask {
    const STATEMENTS: usize = 14;
    const STEP_LIMIT: Duration = Duration::from_secs(60);
    const DEFAULT_COMPILER: &str = "target/debug/knave";
    const FINDINGS_DIRECTORY: &str = "target/agree";
    const FINDINGS_FILE: &str = "findings.txt";

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
        let found = Self::write_findings(workspace_root, &results)?;
        let changes = found.changes_from(&self.expected_findings()?);
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

    fn write_findings(
        workspace_root: &Path,
        results: &RunResults,
    ) -> Result<FindingList, TaskFailure> {
        let directory = workspace_root.join(Self::FINDINGS_DIRECTORY);
        fs::create_dir_all(&directory)
            .map_err(|error| TaskFailure::unwritable(&directory, error))?;
        for finding in &results.findings {
            finding
                .write_program(&directory)
                .map_err(|error| TaskFailure::unwritable(&directory, error))?;
        }
        let found = FindingList::of(&results.findings);
        let summary = directory.join(Self::FINDINGS_FILE);
        found
            .write(&summary)
            .map_err(|error| TaskFailure::unwritable(&summary, error))?;
        print!("{}", found.text());
        Ok(found)
    }

    fn expected_findings(&self) -> Result<FindingList, TaskFailure> {
        match &self.expected {
            Some(path) => {
                FindingList::read(path).map_err(|error| TaskFailure::unreadable(path, error))
            }
            None => Ok(FindingList::default()),
        }
    }
}

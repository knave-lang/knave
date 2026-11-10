mod callgrind;
mod change;
mod comparison;
mod compiler;
mod corpus;
mod counter_comparison;
mod counter_gate;
mod counter_run;
mod failure;
#[cfg(test)]
mod fixture;

mod header;
mod history;
mod measurement;
mod method;
mod output_file;
mod profile;
mod results;
mod table;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Subcommand, ValueEnum};

use crate::failure::TaskFailure;
use crate::git::Git;
use crate::language::LanguageSource;
use crate::workspace::CargoWorkspace;
use callgrind::Callgrind;
use comparison::Comparison;
use compiler::{BuiltCompiler, CompilerBuild};
use corpus::Corpus;
use counter_gate::CounterGate;
use counter_run::CounterRun;
use header::ResultHeader;
use history::BenchHistory;
use method::MeasurementMethod;
use output_file::OutputFile;
use profile::EntryPointCheck;
use results::{Counters, ProgramResult, ResultSet};

#[derive(Subcommand)]
pub enum BenchTask {
    #[command(
        about = "Count the instructions the compiler's front end runs on each corpus program, under callgrind"
    )]
    Measure(MeasureOptions),
    #[command(about = "Print a Markdown table of the change between two saved results")]
    Compare {
        baseline: PathBuf,
        candidate: PathBuf,
        #[arg(long, value_enum, default_value_t = ReportLength::Full)]
        length: ReportLength,
        #[arg(
            long,
            action = clap::ArgAction::Append,
            help = "The programs whose counter rises a `Counters-Accepted:` trailer names"
        )]
        accepted: Vec<String>,
    },
    #[command(
        about = "Fail when a program's allocated bytes or peak live bytes rose past the tolerance, unless `--accepted` names the program"
    )]
    CheckCounters {
        baseline: PathBuf,
        candidate: PathBuf,
        #[arg(
            long,
            action = clap::ArgAction::Append,
            help = "The programs whose counter rises a `Counters-Accepted:` trailer names"
        )]
        accepted: Vec<String>,
    },
    #[command(
        about = "Print the key of what besides the compiler's code decides the counts: the toolchain lock, the dev shell, the benchmark code and the corpus"
    )]
    Method,
    #[command(
        about = "Write a commit's saved results from its git note, if this workspace measures by the same method"
    )]
    Recall(RecallOptions),
}

#[derive(Clone, Copy, ValueEnum)]
pub enum ReportLength {
    #[value(help = "Every stage of every program")]
    Full,
    #[value(
        help = "The number of notable rises and the largest changes, short enough for a comment"
    )]
    Brief,
}

impl ReportLength {
    fn render(self, comparison: &Comparison) -> String {
        match self {
            Self::Full => comparison.full_report(),
            Self::Brief => comparison.brief_report(),
        }
    }
}

#[derive(Args)]
pub struct RecallOptions {
    commit: String,
    #[arg(long, help = "The notes ref that holds the history")]
    notes_ref: String,
    #[arg(long, help = "Write the recalled results to this file for `compare`")]
    save: PathBuf,
}

#[derive(Args)]
pub struct MeasureOptions {
    #[arg(
        long,
        help = "The checkout whose compiler to build and measure [default: this workspace]"
    )]
    workspace: Option<PathBuf>,
    #[arg(
        long,
        help = "The directory of programs to measure [default: the measured checkout's tests/fixtures/corpus]"
    )]
    corpus: Option<PathBuf>,
    #[arg(long, help = "Write the results to this file for `compare`")]
    save: Option<PathBuf>,
}

impl BenchTask {
    pub fn run(self, root: &Path) -> ExitCode {
        let report = match self {
            Self::Measure(options) => FrontEndBenchmark::new(root, options).measure(),
            Self::Compare {
                baseline,
                candidate,
                length,
                accepted,
            } => Self::compare(
                &baseline,
                &candidate,
                length,
                &accepted.into_iter().collect(),
            ),
            Self::CheckCounters {
                baseline,
                candidate,
                accepted,
            } => Self::check_counters(&baseline, &candidate, &accepted.into_iter().collect()),
            Self::Method => MeasurementMethod::new(Git::new(root))
                .key()
                .map(|key| format!("{key}\n")),
            Self::Recall(options) => options.recall(root),
        };
        match report {
            Ok(report) => {
                print!("{report}");
                ExitCode::SUCCESS
            }
            Err(failure) => failure.report(),
        }
    }

    fn compare(
        baseline: &Path,
        candidate: &Path,
        length: ReportLength,
        accepted: &BTreeSet<String>,
    ) -> Result<String, TaskFailure> {
        let baseline = ResultSet::load(baseline)?;
        let candidate = ResultSet::load(candidate)?;
        Ok(length.render(&Comparison::new(&baseline, &candidate, accepted)))
    }

    fn check_counters(
        baseline: &Path,
        candidate: &Path,
        accepted: &BTreeSet<String>,
    ) -> Result<String, TaskFailure> {
        let baseline = ResultSet::load(baseline)?;
        let candidate = ResultSet::load(candidate)?;
        CounterGate::new(&baseline, &candidate, accepted).check()
    }
}

impl RecallOptions {
    fn recall(&self, root: &Path) -> Result<String, TaskFailure> {
        let method_key = MeasurementMethod::new(Git::new(root)).key()?;
        let results = BenchHistory::new(Git::new(root), &self.notes_ref)
            .note_of(&self.commit)?
            .recall(&method_key)?;
        OutputFile::new(&self.save).write_with(|path| fs::write(path, results))?;
        Ok(format!(
            "recalled the results of `{}` into `{}`\n",
            self.commit,
            self.save.display()
        ))
    }
}

impl MeasureOptions {
    fn entry_point_check(&self) -> EntryPointCheck {
        match self.workspace {
            Some(_) => EntryPointCheck::Skipped,
            None => EntryPointCheck::Enforced,
        }
    }
}

type WorkspaceReader = fn(&Path) -> Result<CargoWorkspace, TaskFailure>;

struct FrontEndBenchmark<'a> {
    root: &'a Path,
    options: MeasureOptions,
    read_workspace: WorkspaceReader,
}

#[derive(Debug, PartialEq, Eq)]
struct MeasuredLanguage {
    extension: String,
    name: String,
}

impl<'a> FrontEndBenchmark<'a> {
    const PROFILES: &'static str = "target/bench";
    const EXTENSION_CONSTANT: &'static str = "EXTENSION";
    const NAME_CONSTANT: &'static str = "NAME";
    const FIXED_COMPILER: &'static str = "target/bench/compiler/front-end";
    const TARGET_DIRECTORY: &'static str = "target";

    fn new(root: &'a Path, options: MeasureOptions) -> Self {
        Self {
            root,
            options,
            read_workspace: CargoWorkspace::read,
        }
    }

    #[cfg(test)]
    fn with_workspace_reader(self, read_workspace: WorkspaceReader) -> Self {
        Self {
            read_workspace,
            ..self
        }
    }

    fn measure(&self) -> Result<String, TaskFailure> {
        let saved_header = self.saved_header()?;
        let results = self.measure_corpus()?;
        let report = results.to_string();
        if let (Some(path), Some(header)) = (&self.options.save, saved_header) {
            results.with_header(header).save(path)?;
        }
        Ok(report)
    }

    fn saved_header(&self) -> Result<Option<ResultHeader>, TaskFailure> {
        let method = MeasurementMethod::new(Git::new(self.root));
        match (&self.options.save, &self.options.corpus) {
            (None, _) => Ok(None),
            (Some(_), None) => method.keyed_header().map(Some),
            (Some(_), Some(_)) => method.unkeyed_header().map(Some),
        }
    }

    fn measure_corpus(&self) -> Result<ResultSet, TaskFailure> {
        let language = self.measured_language()?;
        let programs = self.corpus(&language.extension)?.programs()?;
        let measured = self.callgrind()?.measure_all(&programs)?;
        let counted = self.counter_run(language.name)?.count_all(&programs)?;
        Ok(ResultSet::new(Self::attach_counters(measured, counted)))
    }

    fn callgrind(&self) -> Result<Callgrind, TaskFailure> {
        let workspace = self.workspace()?;
        let target_directory = self.root.join(Self::TARGET_DIRECTORY);
        let build = CompilerBuild::new(&workspace, &target_directory).run()?;
        let compiler = self.place_at_fixed_path(build)?;
        Ok(Callgrind::new(
            compiler,
            self.options.entry_point_check(),
            self.root.join(Self::PROFILES),
        ))
    }

    fn counter_run(&self, language_name: String) -> Result<CounterRun, TaskFailure> {
        let workspace = self.workspace()?;
        let target_directory = self.root.join(Self::TARGET_DIRECTORY);
        let build = CompilerBuild::new(&workspace, &target_directory)
            .with_counters_over_plain_build()
            .run()?;
        Ok(CounterRun::new(build.executable, language_name))
    }

    fn attach_counters(measured: Vec<ProgramResult>, counted: Vec<Counters>) -> Vec<ProgramResult> {
        measured
            .into_iter()
            .zip(counted)
            .map(|(result, counters)| ProgramResult { counters, ..result })
            .collect()
    }

    fn workspace(&self) -> Result<PathBuf, TaskFailure> {
        match &self.options.workspace {
            Some(workspace) => Self::absolute(workspace),
            None => Self::absolute(self.root),
        }
    }

    fn measured_language(&self) -> Result<MeasuredLanguage, TaskFailure> {
        let source = LanguageSource::find(&(self.read_workspace)(&self.workspace()?)?)?;
        Ok(MeasuredLanguage {
            extension: source.constant(Self::EXTENSION_CONSTANT)?,
            name: source.constant(Self::NAME_CONSTANT)?,
        })
    }

    fn corpus(&self, extension: &str) -> Result<Corpus, TaskFailure> {
        let directory = match &self.options.corpus {
            Some(corpus) => Self::absolute(corpus)?,
            None => self.workspace()?.join(Corpus::DIRECTORY),
        };
        Ok(Corpus::new(directory, extension.to_string()))
    }

    fn place_at_fixed_path(&self, compiler: BuiltCompiler) -> Result<BuiltCompiler, TaskFailure> {
        let placed = self.root.join(Self::FIXED_COMPILER);
        OutputFile::new(&placed).write_with(|path| fs::copy(&compiler.executable, path))?;
        Ok(BuiltCompiler {
            executable: placed,
            ..compiler
        })
    }

    fn absolute(path: &Path) -> Result<PathBuf, TaskFailure> {
        path.canonicalize()
            .map_err(|error| TaskFailure::unreadable(path, error))
    }
}

#[cfg(test)]
mod tests {
    use clap::{Command, FromArgMatches};

    use super::*;
    use crate::workspace::WorkspaceFixture;

    struct BenchArgumentsFixture;

    impl BenchArgumentsFixture {
        const COMMAND: &'static str = "bench";

        fn accepted(arguments: &[&str]) -> Vec<String> {
            match Self::parse(arguments) {
                BenchTask::Compare { accepted, .. } | BenchTask::CheckCounters { accepted, .. } => {
                    accepted
                }
                _ => panic!("{arguments:?} parsed to another task"),
            }
        }

        fn parse(arguments: &[&str]) -> BenchTask {
            let command = BenchTask::augment_subcommands(Command::new(Self::COMMAND));
            let matches = command
                .try_get_matches_from([Self::COMMAND].iter().chain(arguments))
                .unwrap();
            BenchTask::from_arg_matches(&matches).unwrap()
        }
    }

    struct MeasuredCheckoutFixture;

    impl MeasuredCheckoutFixture {
        fn benchmark(root: &Path, measured: PathBuf) -> FrontEndBenchmark<'_> {
            let options = MeasureOptions {
                workspace: Some(measured),
                corpus: None,
                save: None,
            };
            FrontEndBenchmark::new(root, options).with_workspace_reader(Self::read_workspace)
        }

        fn read_workspace(path: &Path) -> Result<CargoWorkspace, TaskFailure> {
            let fixture = WorkspaceFixture::root().canonicalize().unwrap();
            if path == fixture {
                Ok(WorkspaceFixture::workspace())
            } else {
                CargoWorkspace::read(path)
            }
        }
    }

    #[test]
    fn a_measured_checkout_supplies_its_own_extension_and_name() {
        let repository = WorkspaceFixture::repository_root();
        let benchmark = MeasuredCheckoutFixture::benchmark(repository, WorkspaceFixture::root());
        assert_eq!(
            benchmark.measured_language().unwrap(),
            MeasuredLanguage {
                extension: "ldg".to_string(),
                name: "ledger".to_string(),
            }
        );
    }

    #[test]
    fn a_measured_checkout_runs_its_own_corpus() {
        let repository = WorkspaceFixture::repository_root();
        let measured = repository.join("xtask");
        let benchmark = MeasuredCheckoutFixture::benchmark(repository, measured.clone());
        let corpus = benchmark.corpus("knv").unwrap();
        assert_eq!(
            corpus.directory(),
            measured.canonicalize().unwrap().join(Corpus::DIRECTORY)
        );
    }

    #[test]
    fn check_counters_accepts_no_program_by_default() {
        let accepted = BenchArgumentsFixture::accepted(&["check-counters", "a", "b"]);
        assert!(accepted.is_empty());
    }

    #[test]
    fn check_counters_collects_each_accepted_program() {
        let accepted = BenchArgumentsFixture::accepted(&[
            "check-counters",
            "a",
            "b",
            "--accepted=warehouse",
            "--accepted=orders",
        ]);
        assert_eq!(accepted, ["warehouse", "orders"]);
    }

    #[test]
    fn check_counters_takes_an_accepted_program_that_looks_like_a_flag() {
        let accepted =
            BenchArgumentsFixture::accepted(&["check-counters", "a", "b", "--accepted=--help"]);
        assert_eq!(accepted, ["--help"]);
    }

    #[test]
    fn compare_accepts_no_program_by_default() {
        let accepted = BenchArgumentsFixture::accepted(&["compare", "a", "b"]);
        assert!(accepted.is_empty());
    }

    #[test]
    fn compare_collects_each_accepted_program() {
        let accepted = BenchArgumentsFixture::accepted(&[
            "compare",
            "a",
            "b",
            "--accepted=warehouse",
            "--accepted=orders",
        ]);
        assert_eq!(accepted, ["warehouse", "orders"]);
    }

    #[test]
    fn compare_takes_an_accepted_program_that_looks_like_a_flag() {
        let accepted = BenchArgumentsFixture::accepted(&["compare", "a", "b", "--accepted=--help"]);
        assert_eq!(accepted, ["--help"]);
    }
}

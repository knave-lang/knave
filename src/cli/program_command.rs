use std::path::PathBuf;

use clap::{Args, Subcommand};

use super::build::BuildCommand;
use super::entry_help::ENTRY_HELP;
use super::eval::{EvalCommand, EvaluationSettings, FormsText, ModuleText};
use super::evaluation_output::{EvaluationOutput, JsonEvaluation, PrintedEvaluation};
use super::exit_status::ExitStatus;
use super::release_flag::ReleaseFlag;
use super::repl::ReplCommand;
use super::run::RunCommand;
use super::terminal::{BuildOptions, Terminal};
use super::test::{TestCommand, TestSelection};
use crate::source::TextPosition;

#[derive(Subcommand)]
pub enum ProgramCommand {
    #[command(about = "Compile a program to a native binary")]
    Build {
        #[arg(help = ENTRY_HELP)]
        file: Option<PathBuf>,
        #[command(flatten)]
        release: ReleaseFlag,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    #[command(about = "Check a module, then run forms once in its scope and print their values")]
    Eval {
        file: PathBuf,
        #[arg(allow_hyphen_values = true)]
        forms: String,
        #[command(flatten)]
        stdin: StdinFlag,
        #[arg(
            long,
            help = "Resolve `use` from this directory [default: the project's source root, or the file's directory]"
        )]
        root: Option<PathBuf>,
        #[command(flatten)]
        position: PositionFlags,
        #[command(flatten)]
        release: ReleaseFlag,
        #[command(flatten)]
        json: JsonFlags,
    },
    #[command(about = "Read forms at a prompt, keep the definitions and run the rest")]
    Repl {
        #[arg(
            long,
            help = "Resolve `use` from this directory [default: the project's source root, or the working directory]"
        )]
        root: Option<PathBuf>,
        #[command(flatten)]
        release: ReleaseFlag,
    },
    #[command(about = "Build the tests of a file or a directory and run them")]
    Test {
        #[arg(help = "The test file or directory [default: the project's `tests` directory]")]
        path: Option<PathBuf>,
        #[arg(hide = true)]
        positional_filter: Option<String>,
        #[arg(
            long,
            value_name = "TEXT",
            help = "Run only the tests whose full name contains this text"
        )]
        filter: Option<String>,
        #[arg(
            long,
            value_name = "NAME",
            conflicts_with = "filter",
            help = "Run only the test with this full name"
        )]
        exact: Option<String>,
        #[arg(
            long,
            value_name = "MODULE",
            help = "List the names that MODULE exports and no test writes"
        )]
        untested: Option<String>,
        #[command(flatten)]
        release: ReleaseFlag,
    },
    #[command(about = "Build a program and run it")]
    Run {
        #[arg(help = ENTRY_HELP)]
        file: Option<PathBuf>,
        #[command(flatten)]
        release: ReleaseFlag,
        #[arg(last = true)]
        args: Vec<String>,
    },
}

impl ProgramCommand {
    pub fn execute(self, terminal: &mut Terminal) -> ExitStatus {
        match self {
            Self::Build {
                file,
                release,
                output,
            } => BuildCommand::new(
                file,
                BuildOptions {
                    profile: release.profile(),
                    output,
                },
            )
            .run(terminal),
            Self::Eval {
                file,
                forms,
                stdin,
                root,
                position,
                release,
                json,
            } => EvalCommand::new(
                stdin.module_text(file),
                root,
                FormsText::new(forms, position.position()),
                EvaluationSettings {
                    profile: release.profile(),
                    output: json.output(),
                },
            )
            .run(terminal),
            Self::Repl { root, release } => ReplCommand::new(root, release.profile()).run(terminal),
            Self::Test {
                positional_filter: Some(filter),
                ..
            } => TestSelection::report_positional_filter(&filter),
            Self::Test {
                path,
                positional_filter: None,
                filter,
                exact,
                untested,
                release,
            } => TestCommand::new(
                path,
                TestSelection::of(filter, exact),
                untested,
                release.profile(),
            )
            .run(terminal),
            Self::Run {
                file,
                release,
                args,
            } => RunCommand::new(
                file,
                BuildOptions {
                    profile: release.profile(),
                    output: None,
                },
                args,
            )
            .run(terminal),
        }
    }
}

#[derive(Args)]
pub struct StdinFlag {
    #[arg(long, help = "Read the module's text from standard input")]
    stdin: bool,
}

#[derive(Args)]
pub struct PositionFlags {
    #[arg(
        long,
        requires = "column",
        help = "The line where the forms start in the module's text"
    )]
    line: Option<usize>,
    #[arg(
        long,
        requires = "line",
        help = "The byte column where the forms start, from 1"
    )]
    column: Option<usize>,
}

#[derive(Args)]
pub struct JsonFlags {
    #[arg(
        long,
        help = "Write one JSON object per line for each output, value and diagnostic"
    )]
    json: bool,
    #[arg(
        long,
        requires = "json",
        help = "Echo this text as `id` on every JSON object"
    )]
    id: Option<String>,
}

impl StdinFlag {
    fn module_text(&self, file: PathBuf) -> ModuleText {
        if self.stdin {
            ModuleText::StandardInput(file)
        } else {
            ModuleText::File(file)
        }
    }
}

impl PositionFlags {
    fn position(&self) -> Option<TextPosition> {
        Some(TextPosition::new(self.line?, self.column?))
    }
}

impl JsonFlags {
    fn output(self) -> Box<dyn EvaluationOutput> {
        if self.json {
            Box::new(JsonEvaluation::new(self.id))
        } else {
            Box::new(PrintedEvaluation::for_standard_output())
        }
    }
}

use std::path::PathBuf;

use clap::Subcommand;

use super::check::CheckCommand;
use super::check_flag::CheckFlag;
use super::doc::DocCommand;
use super::doc_arguments::DocArguments;
use super::emit::EmitCommand;
use super::entry_help::ENTRY_HELP;
use super::exit_status::ExitStatus;
use super::expand::ExpandCommand;
use super::fmt::FmtCommand;
use super::terminal::Terminal;
use super::types::TypesCommand;

#[derive(Subcommand)]
pub enum InspectionCommand {
    #[command(about = "Read, expand and check a program, then print its diagnostics")]
    Check {
        #[arg(help = ENTRY_HELP)]
        file: Option<PathBuf>,
    },
    #[command(about = "Print the inferred signature of each function in a program")]
    Types {
        #[arg(help = ENTRY_HELP)]
        file: Option<PathBuf>,
    },
    #[command(
        name = DocCommand::NAME,
        about = "Print the page of a prelude term, or of a program's term with --file"
    )]
    Doc(DocArguments),
    #[command(about = "Print a program as kernel forms after macro expansion")]
    Expand {
        #[arg(help = ENTRY_HELP)]
        file: Option<PathBuf>,
    },
    #[command(about = "Print the Rust code generated for a program")]
    Emit {
        #[arg(help = ENTRY_HELP)]
        file: Option<PathBuf>,
    },
    #[command(about = "Lay out source files in place, or standard input given `-`")]
    Fmt {
        #[arg(help = "The files and directories to lay out [default: the project's root]")]
        paths: Vec<PathBuf>,
        #[command(flatten)]
        check: CheckFlag,
        #[arg(
            long,
            value_name = "FILE",
            help = "Find the modules that standard input uses as if it were this file"
        )]
        stdin_path: Option<PathBuf>,
    },
}

impl InspectionCommand {
    pub fn execute(self, terminal: &mut Terminal) -> ExitStatus {
        match self {
            Self::Check { file } => CheckCommand::new(file).run(terminal),
            Self::Types { file } => TypesCommand::new(file).run(terminal),
            Self::Doc(arguments) => arguments.run(terminal),
            Self::Expand { file } => ExpandCommand::new(file).run(terminal),
            Self::Emit { file } => EmitCommand::new(file).run(terminal),
            Self::Fmt {
                paths,
                check,
                stdin_path,
            } => FmtCommand::new(paths, check.mode(), stdin_path).run(terminal),
        }
    }
}

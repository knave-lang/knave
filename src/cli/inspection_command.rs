use std::path::PathBuf;

use clap::Subcommand;

use super::check::CheckCommand;
use super::check_flag::CheckFlag;
use super::doc::DocCommand;
use super::doc_arguments::DocArguments;
use super::emit::EmitCommand;
use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::expand::ExpandCommand;
use super::fmt::FmtCommand;
use super::terminal::Terminal;
use super::types::TypesCommand;
use crate::ownership::{EnforcedOwnership, WaivedOwnership};

#[derive(Subcommand)]
pub enum InspectionCommand {
    #[command(about = "Read, expand and check a program, then print its diagnostics")]
    Check {
        #[command(flatten)]
        entry: EntryArgument,
    },
    #[command(about = "Print the inferred signature of each function in a program")]
    Types {
        #[command(flatten)]
        entry: EntryArgument,
    },
    #[command(
        name = DocCommand::NAME,
        about = "Print the page of a prelude term, or of a program's term with --file"
    )]
    Doc(DocArguments),
    #[command(about = "Print a program as kernel forms after macro expansion")]
    Expand {
        #[command(flatten)]
        entry: EntryArgument,
    },
    #[command(about = "Print the Rust code generated for a program")]
    Emit {
        #[command(flatten)]
        entry: EntryArgument,
        #[arg(long, hide = true)]
        waive_ownership_refusals: bool,
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
            Self::Check { entry } => CheckCommand::new(entry).run(terminal),
            Self::Types { entry } => TypesCommand::new(entry).run(terminal),
            Self::Doc(arguments) => arguments.run(terminal),
            Self::Expand { entry } => ExpandCommand::new(entry).run(terminal),
            Self::Emit {
                entry,
                waive_ownership_refusals: false,
            } => EmitCommand::<EnforcedOwnership>::new(entry).run(terminal),
            Self::Emit {
                entry,
                waive_ownership_refusals: true,
            } => EmitCommand::<WaivedOwnership>::new(entry).run(terminal),
            Self::Fmt {
                paths,
                check,
                stdin_path,
            } => FmtCommand::new(paths, check.mode(), stdin_path).run(terminal),
        }
    }
}

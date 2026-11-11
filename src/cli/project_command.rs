use clap::Subcommand;

use super::add::AddCommand;
use super::crate_section::CrateSection;
use super::exit_status::ExitStatus;
use super::new::NewCommand;
use super::release_flag::ReleaseFlag;
use super::remove::RemoveCommand;
use super::task::TaskCommand;
use super::terminal::Terminal;

const DEV_HELP: &str = "Edit `dev-dependencies`, which only `test` builds with";

#[derive(Subcommand)]
pub enum ProjectCommand {
    #[command(about = "Add a crate to the project's dependencies, at the version Cargo picks")]
    Add {
        #[arg(help = "The crate's name, as crates.io writes it")]
        name: String,
        #[arg(
            long,
            value_delimiter = ',',
            help = "Features to turn on, separated by commas"
        )]
        features: Vec<String>,
        #[arg(long = CrateSection::DEV_FLAG_NAME, help = DEV_HELP)]
        dev: bool,
    },
    #[command(about = "Remove a crate from the project's dependencies")]
    Remove {
        #[arg(help = "The crate's name, as the project file writes it")]
        name: String,
        #[arg(long = CrateSection::DEV_FLAG_NAME, help = DEV_HELP)]
        dev: bool,
    },
    #[command(
        name = TaskCommand::NAME,
        about = "Run a task from the project's `tasks` directory, or list the tasks"
    )]
    Task {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            value_name = "NAME [ARGUMENTS]",
            help = "The task's name, then the arguments it takes [default: list the tasks]"
        )]
        command: Vec<String>,
        #[command(flatten)]
        release: ReleaseFlag,
    },
    #[command(about = "Create a project in a new directory of its name")]
    New {
        #[arg(
            help = "The project's name, in kebab-case",
            value_parser = clap::builder::NonEmptyStringValueParser::new()
        )]
        name: String,
    },
}

impl ProjectCommand {
    pub fn execute(self, terminal: &mut Terminal) -> ExitStatus {
        match self {
            Self::New { name } => NewCommand::new(name).run(terminal),
            Self::Add {
                name,
                features,
                dev,
            } => AddCommand::new(name, features, CrateSection::of(dev)).run(terminal),
            Self::Remove { name, dev } => {
                RemoveCommand::new(name, CrateSection::of(dev)).run(terminal)
            }
            Self::Task { command, release } => {
                TaskCommand::new(command, release.profile()).run(terminal)
            }
        }
    }
}

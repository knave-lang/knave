use clap::Subcommand;

use super::exit_status::ExitStatus;
use super::inspection_command::InspectionCommand;
use super::program_command::ProgramCommand;
use super::project_command::ProjectCommand;
use super::release_flag::ReleaseFlag;
use super::terminal::Terminal;

#[derive(Subcommand)]
pub enum Command {
    #[command(flatten)]
    Inspection(InspectionCommand),
    #[command(flatten)]
    Program(ProgramCommand),
    #[command(flatten)]
    Project(ProjectCommand),
}

impl Command {
    pub fn prompt() -> Self {
        Self::Program(ProgramCommand::Repl {
            root: None,
            release: ReleaseFlag::default(),
        })
    }

    pub fn execute(self, terminal: &mut Terminal) -> ExitStatus {
        match self {
            Command::Inspection(command) => command.execute(terminal),
            Command::Program(command) => command.execute(terminal),
            Command::Project(command) => command.execute(terminal),
        }
    }
}

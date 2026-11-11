use std::path::PathBuf;

use clap::Args;

use super::exit_status::ExitStatus;
use super::located_program::LocatedProgram;
use super::terminal::Terminal;

#[derive(Args)]
pub struct EntryArgument {
    #[arg(help = "The program's entry file [default: the project's `:entry`]")]
    file: Option<PathBuf>,
}

impl EntryArgument {
    pub fn locate(self, terminal: &Terminal) -> Result<LocatedProgram, ExitStatus> {
        match self.file {
            Some(file) => LocatedProgram::of_file(file, terminal),
            None => LocatedProgram::of_working_directory(terminal),
        }
    }

    pub fn defaults_to_project_entry(&self) -> bool {
        self.file.is_none()
    }
}

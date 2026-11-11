use std::path::PathBuf;

use clap::Args;

#[derive(Args)]
pub struct EntryArgument {
    #[arg(help = "The program's entry file [default: the project's `:entry`]")]
    file: Option<PathBuf>,
}

impl EntryArgument {
    pub fn into_file(self) -> Option<PathBuf> {
        self.file
    }
}

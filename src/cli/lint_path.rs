use std::path::{Path, PathBuf};

use clap::Args;

#[derive(Args)]
pub struct LintPath {
    #[arg(help = "A file or a directory to lint [default: every file of the project]")]
    path: Option<PathBuf>,
}

impl LintPath {
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

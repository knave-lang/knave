use std::path::{Path, PathBuf};

use super::exit_status::ExitStatus;
use super::standard_error::StandardError;
use crate::core::ModulePath;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct TreeFile {
    module: String,
    path: PathBuf,
}

impl TreeFile {
    const UNNAMABLE: [char; 4] = ['(', ')', '"', ';'];

    pub fn new(module: String, path: PathBuf) -> Self {
        Self { module, path }
    }

    pub fn into_module(self) -> String {
        self.module
    }

    pub fn is_unnamable(&self) -> bool {
        self.module
            .chars()
            .any(|character| character.is_whitespace() || Self::UNNAMABLE.contains(&character))
    }

    pub fn report_unnamable(&self) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: `{}` cannot be a module, because a module path cannot hold a space, a parenthesis, a quote or a semicolon\n\
             help: rename the file",
            self.path.display()
        ));
        ExitStatus::UsageError
    }

    pub fn module_path(&self) -> ModulePath {
        ModulePath::new(self.module.as_str())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

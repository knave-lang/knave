use std::path::Path;
use std::process::{Command, Output};

use crate::failure::TaskFailure;

#[derive(Clone, Copy)]
pub struct Git<'a> {
    workspace: &'a Path,
}

impl<'a> Git<'a> {
    pub const PROGRAM: &'static str = "git";
    const LOCALE_VARIABLE: &'static str = "LC_ALL";
    const UNTRANSLATED_LOCALE: &'static str = "C";
    const INDEX_VARIABLE: &'static str = "GIT_INDEX_FILE";

    pub fn new(workspace: &'a Path) -> Self {
        Self { workspace }
    }

    pub fn run(&self, arguments: &[&str]) -> Result<Output, TaskFailure> {
        self.command(arguments)
            .output()
            .map_err(|error| TaskFailure::tool_missing(Self::PROGRAM, error))
    }

    pub fn run_with_index(&self, index: &Path, arguments: &[&str]) -> Result<Output, TaskFailure> {
        self.command(arguments)
            .env(Self::INDEX_VARIABLE, index)
            .output()
            .map_err(|error| TaskFailure::tool_missing(Self::PROGRAM, error))
    }

    pub fn merge_base(&self, revision: &str) -> Result<String, TaskFailure> {
        let output = self.run(&["merge-base", revision, "HEAD"])?;
        if !output.status.success() {
            return Err(TaskFailure::unknown_revision(revision));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn command(&self, arguments: &[&str]) -> Command {
        let mut command = Command::new(Self::PROGRAM);
        command
            .args(arguments)
            .env(Self::LOCALE_VARIABLE, Self::UNTRANSLATED_LOCALE)
            .current_dir(self.workspace);
        command
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn pins_the_untranslated_locale() {
        let command = Git::new(Path::new("/checkout")).command(&["status"]);
        let locale = command
            .get_envs()
            .find(|(variable, _)| *variable == OsStr::new(Git::LOCALE_VARIABLE))
            .and_then(|(_, value)| value);
        assert_eq!(locale, Some(OsStr::new(Git::UNTRANSLATED_LOCALE)));
    }
}

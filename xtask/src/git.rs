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

    pub fn succeed(&self, arguments: &[&str]) -> Result<Vec<u8>, TaskFailure> {
        Self::standard_output(self.run(arguments)?, arguments)
    }

    pub fn succeed_with_index(
        &self,
        index: &Path,
        arguments: &[&str],
    ) -> Result<Vec<u8>, TaskFailure> {
        let output = self
            .command(arguments)
            .env(Self::INDEX_VARIABLE, index)
            .output()
            .map_err(|error| TaskFailure::tool_missing(Self::PROGRAM, error))?;
        Self::standard_output(output, arguments)
    }

    pub fn merge_base(&self, revision: &str) -> Result<String, TaskFailure> {
        let output = self.run(&["merge-base", revision, "HEAD"])?;
        if !output.status.success() {
            return Err(TaskFailure::unknown_revision(revision));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn standard_output(output: Output, arguments: &[&str]) -> Result<Vec<u8>, TaskFailure> {
        if output.status.success() {
            Ok(output.stdout)
        } else {
            let subcommand = arguments.first().copied().unwrap_or_default();
            let tool = format!("{} {subcommand}", Self::PROGRAM);
            Err(TaskFailure::tool_failed(&tool, &output.stderr))
        }
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

    #[test]
    fn a_failed_command_names_its_subcommand() {
        let failure = Git::new(Path::new(env!("CARGO_MANIFEST_DIR")))
            .succeed(&["rev-parse", "--verify", "no-such-revision"])
            .expect_err("the revision is unknown");
        assert!(failure.to_string().contains("git rev-parse"), "{failure}");
    }
}

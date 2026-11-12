use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::emitted_rust::EmittedRust;
use crate::knave_standard_error::KnaveStandardError;
use crate::scratch_directory::ScratchDirectory;
use crate::subject_program::SubjectProgram;
use crate::timed_command::{Completion, Finished, TimedCommand};
use crate::tool_run::ToolRun;
use crate::top_level_definitions::TopLevelDefinitions;
use crate::verdict::Verdict;

pub struct KnaveInvocation<'a> {
    binary: &'a Path,
    program: &'a SubjectProgram,
    directory: &'a ScratchDirectory,
}

impl<'a> KnaveInvocation<'a> {
    const TOOL: &'static str = "knave";
    const PROGRAM_ERRORS: i32 = 1;
    const CHECK: &'static str = "check";
    const EMIT: &'static str = "emit";
    const EXPAND: &'static str = "expand";
    const WAIVE_OWNERSHIP_REFUSALS: &'static str = "--waive-ownership-refusals";
    const NO_COLOR: &'static str = "NO_COLOR";

    pub fn new(
        binary: &'a Path,
        program: &'a SubjectProgram,
        directory: &'a ScratchDirectory,
    ) -> Self {
        Self {
            binary,
            program,
            directory,
        }
    }

    pub fn check(&self, limit: Duration) -> Result<(), Verdict> {
        self.run(&[Self::CHECK], limit).map(|_| ())
    }

    pub fn emit(&self, limit: Duration) -> Result<EmittedRust, Verdict> {
        self.run(&[Self::EMIT], limit)
            .map(|finished| EmittedRust::of(finished, self.program))
    }

    pub fn emit_waiving_ownership_refusals(&self, limit: Duration) -> Result<EmittedRust, Verdict> {
        self.run(&[Self::EMIT, Self::WAIVE_OWNERSHIP_REFUSALS], limit)
            .map(|finished| EmittedRust::of(finished, self.program))
    }

    fn run(&self, arguments: &[&str], limit: Duration) -> Result<Finished, Verdict> {
        let finished = ToolRun::of(Self::TOOL).finished(self.completion(arguments, limit))?;
        if finished.status.success() {
            Ok(finished)
        } else {
            Err(self.refused(&finished, limit))
        }
    }

    fn completion(&self, arguments: &[&str], limit: Duration) -> std::io::Result<Completion> {
        let mut command = Command::new(self.binary);
        command
            .args(arguments)
            .arg(self.program.entry.as_os_str())
            .current_dir(self.directory.path())
            .env(Self::NO_COLOR, OsStr::new("1"));
        TimedCommand::new(command).run(limit)
    }

    fn refused(&self, finished: &Finished, limit: Duration) -> Verdict {
        let stderr = String::from_utf8_lossy(&finished.stderr);
        match finished.status.code() {
            Some(Self::PROGRAM_ERRORS) => Verdict::Rejected(
                KnaveStandardError::of(self.program, self.expanded_definitions(limit))
                    .rejections(&stderr),
            ),
            _ => Verdict::Unsupported(format!("knave failed: {stderr}")),
        }
    }

    fn expanded_definitions(&self, limit: Duration) -> TopLevelDefinitions {
        match self.completion(&[Self::EXPAND], limit) {
            Ok(Completion::Finished(expanded)) if expanded.status.success() => {
                TopLevelDefinitions::of_text(&String::from_utf8_lossy(&expanded.stdout))
            }
            _ => self.program.definitions(),
        }
    }
}

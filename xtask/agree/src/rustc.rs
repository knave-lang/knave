use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use knave_expand::diagnostic::DiagnosticCode;
use serde_json::Value;

use crate::debug_binary::DebugBinary;
use crate::emitted_rust::EmittedRust;
use crate::rust_functions::RustFunctions;
use crate::timed_command::TimedCommand;
use crate::tool_run::ToolRun;
use crate::verdict::{RejectingTool, Rejection, Verdict};

pub struct Rustc<'a> {
    directory: &'a Path,
    emitted: &'a EmittedRust,
}

impl<'a> Rustc<'a> {
    const TOOL: &'static str = "rustc";
    const SOURCE_FILE: &'static str = "main.rs";
    const BINARY_FILE: &'static str = "program";
    const COMMON_ARGUMENTS: [&'static str; 8] = [
        "--edition",
        "2024",
        "--crate-name",
        "program",
        "--error-format=json",
        "--cap-lints",
        "allow",
        "-Cdebuginfo=0",
    ];
    const DEBUG_PROFILE: [&'static str; 2] = ["-Cdebug-assertions=on", "-Coverflow-checks=on"];
    const METADATA_ONLY: [&'static str; 1] = ["--emit=metadata"];

    pub fn new(directory: &'a Path, emitted: &'a EmittedRust) -> Self {
        Self { directory, emitted }
    }

    pub fn analyze(&self, limit: Duration) -> Verdict {
        match self.compile(&Self::METADATA_ONLY, limit) {
            Ok(()) => Verdict::Accepted,
            Err(verdict) => verdict,
        }
    }

    pub fn build_debug(&self, limit: Duration) -> Result<DebugBinary, Verdict> {
        self.compile(&Self::DEBUG_PROFILE, limit)
            .map(|()| DebugBinary::at(self.binary()))
    }

    fn binary(&self) -> PathBuf {
        self.directory.join(Self::BINARY_FILE)
    }

    fn compile(&self, profile: &[&str], limit: Duration) -> Result<(), Verdict> {
        let source = self.directory.join(Self::SOURCE_FILE);
        fs::write(&source, self.emitted.text()).map_err(|error| {
            Verdict::Unsupported(format!("cannot write the emitted Rust: {error}"))
        })?;
        let mut command = Command::new(Self::TOOL);
        command
            .args(Self::COMMON_ARGUMENTS)
            .args(profile)
            .arg("-o")
            .arg(self.binary())
            .arg(&source)
            .current_dir(self.directory);
        let finished = ToolRun::of(Self::TOOL).finished(TimedCommand::new(command).run(limit))?;
        if finished.status.success() {
            Ok(())
        } else {
            Err(Verdict::Rejected(
                self.rejections(&String::from_utf8_lossy(&finished.stderr)),
            ))
        }
    }

    fn rejections(&self, json_lines: &str) -> Vec<Rejection> {
        let functions = RustFunctions::of_text(self.emitted.text());
        json_lines
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(|message| message["level"] == "error")
            .filter_map(|message| self.rejection(&message, &functions))
            .collect()
    }

    fn rejection(&self, message: &Value, functions: &RustFunctions) -> Option<Rejection> {
        let primary = message["spans"]
            .as_array()?
            .iter()
            .find(|span| span["is_primary"] == true)?;
        let start = usize::try_from(primary["byte_start"].as_u64()?).ok()?;
        let end = usize::try_from(primary["byte_end"].as_u64()?).ok()?;
        Some(Rejection {
            tool: RejectingTool::Rustc,
            code: message["code"]["code"]
                .as_str()
                .map(|code| DiagnosticCode::from_text(code.to_string())),
            file: PathBuf::from(primary["file_name"].as_str()?),
            span: start..end,
            reached_from_main: true,
            function: functions
                .name_at(start)
                .map(|rust_name| self.emitted.knave_name(rust_name)),
        })
    }
}

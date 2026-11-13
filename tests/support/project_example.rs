use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use knave::language::Language;

use super::ExampleDiagnostic;

pub struct ProjectExample {
    command: Option<Vec<String>>,
    files: Vec<ExampleFile>,
    input: String,
}

struct ExampleFolder {
    path: PathBuf,
}

impl ExampleFolder {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ExampleFolder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct ExampleFile {
    path: String,
    text: String,
}

impl ProjectExample {
    const COMPILER: &str = env!("CARGO_BIN_EXE_knave");
    const LABEL: &str = ";; ";
    const FILE_EXTENSION: &str = ".knv";
    const COMPILER_NAME: &str = "knave";
    const SUBCOMMANDS: [&str; 15] = [
        "check", "types", "doc", "expand", "emit", "fmt", "build", "eval", "repl", "test", "run",
        "add", "remove", "task", "new",
    ];
    const PLAIN_OUTPUT: [&str; 2] = ["--color", "never"];
    const CHECK: &str = "check";
    const BUILD: &str = "build";
    const RUN: &str = "run";
    const DIAGNOSTIC_LEVELS: [&str; 2] = ["error[", "warning["];

    pub fn parse(block: &str) -> Option<Self> {
        let mut lines = block.lines().peekable();
        let command = lines
            .next_if(|line| Self::command_of(line).is_some())
            .and_then(Self::command_of);
        let mut example = Self {
            command,
            files: Vec::new(),
            input: String::new(),
        };
        for line in lines {
            example.take_line(line);
        }
        example.refuse_text_above_the_first_file();
        let marked = example.command.is_some() || !example.files.is_empty();
        marked.then_some(example)
    }

    pub fn reported_diagnostics(&self) -> Vec<ExampleDiagnostic> {
        let folder = self.written_folder();
        let stages: Vec<Vec<String>> = match &self.command {
            Some(command) => vec![command.clone()],
            None => vec![vec![Self::CHECK.to_string()], vec![Self::BUILD.to_string()]],
        };
        stages
            .iter()
            .map(|command| Self::diagnostics_in(&self.run(folder.path(), command)))
            .find(|diagnostics| {
                diagnostics
                    .iter()
                    .any(|diagnostic| !diagnostic.code.is_empty())
            })
            .unwrap_or_default()
    }

    pub fn failure(&self) -> Option<String> {
        let folder = self.written_folder();
        let command = self
            .command
            .clone()
            .unwrap_or_else(|| vec![Self::RUN.to_string()]);
        let output = self.run(folder.path(), &command);
        let reports_a_code = !Self::diagnostics_in(&output).is_empty();
        let failed = !output.status.success() || reports_a_code;
        failed.then(|| String::from_utf8_lossy(&output.stderr).into_owned())
    }

    fn refuse_text_above_the_first_file(&self) {
        let text_above = self.input.lines().any(|line| !line.trim().is_empty());
        assert!(
            self.files.is_empty() || !text_above,
            "an explanation example has text above its first file label:\n{}",
            self.input
        );
    }

    fn take_line(&mut self, line: &str) {
        match Self::file_of(line) {
            Some(path) => self.files.push(ExampleFile {
                path: path.to_string(),
                text: String::new(),
            }),
            None => self.append(line),
        }
    }

    fn file_of(line: &str) -> Option<&str> {
        let path = line.strip_prefix(Self::LABEL)?;
        let only_a_path = !path.is_empty() && !path.contains(char::is_whitespace);
        (only_a_path && path.ends_with(Self::FILE_EXTENSION)).then_some(path)
    }

    fn command_of(line: &str) -> Option<Vec<String>> {
        let mut words = line.strip_prefix(Self::LABEL)?.split_whitespace();
        if words.next() != Some(Self::COMPILER_NAME) {
            return None;
        }
        let command: Vec<String> = words.map(str::to_string).collect();
        let subcommand = command.first()?;
        Self::SUBCOMMANDS
            .contains(&subcommand.as_str())
            .then_some(command)
    }

    fn append(&mut self, line: &str) {
        let target = match self.files.last_mut() {
            Some(file) => &mut file.text,
            None => &mut self.input,
        };
        target.push_str(line);
        target.push('\n');
    }

    fn written_folder(&self) -> ExampleFolder {
        static NEXT_EXAMPLE: AtomicUsize = AtomicUsize::new(0);
        let example = NEXT_EXAMPLE.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "{}-example-{}-{example}",
            Language::NAME,
            std::process::id()
        ));
        for file in &self.files {
            let path = directory.join(&file.path);
            fs::create_dir_all(path.parent().expect("a file lies in a folder"))
                .expect("the example folder is writable");
            fs::write(path, &file.text).expect("the example file is writable");
        }
        fs::create_dir_all(&directory).expect("the example folder is writable");
        ExampleFolder { path: directory }
    }

    fn run(&self, directory: &Path, command: &[String]) -> Output {
        let mut child = Command::new(Self::COMPILER)
            .args(Self::PLAIN_OUTPUT)
            .args(command)
            .current_dir(directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the compiler starts");
        child
            .stdin
            .take()
            .expect("the compiler's input is piped")
            .write_all(self.input.as_bytes())
            .expect("the compiler reads its input");
        child.wait_with_output().expect("the compiler finishes")
    }

    fn diagnostics_in(output: &Output) -> Vec<ExampleDiagnostic> {
        let printed = [&output.stderr, &output.stdout]
            .map(|stream| String::from_utf8_lossy(stream).into_owned())
            .concat();
        printed.lines().filter_map(Self::diagnostic_of).collect()
    }

    fn diagnostic_of(line: &str) -> Option<ExampleDiagnostic> {
        let rest = Self::DIAGNOSTIC_LEVELS
            .iter()
            .find_map(|level| line.strip_prefix(level))?;
        let (code, title) = rest.split_once("]: ")?;
        Some(ExampleDiagnostic {
            code: code.to_string(),
            title: title.to_string(),
        })
    }
}

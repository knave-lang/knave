use std::process::Output;

use super::ExampleDiagnostic;
use super::example_folder::{ExampleFile, ExampleFolder};
use super::example_layout::ExampleLayout;

pub struct ProjectExample {
    command_line: Option<String>,
    command: Option<Vec<String>>,
    files: Vec<ExampleFile>,
    input: String,
}

impl ProjectExample {
    const LABEL: &str = ";; ";
    const FILE_EXTENSION: &str = ".knv";
    const COMPILER_NAME: &str = "knave";
    const SUBCOMMANDS: [&str; 15] = [
        "check", "types", "doc", "expand", "emit", "fmt", "build", "eval", "repl", "test", "run",
        "add", "remove", "task", "new",
    ];
    const CHECK: &str = "check";
    const BUILD: &str = "build";
    const RUN: &str = "run";
    const DIAGNOSTIC_LEVELS: [&str; 2] = ["error[", "warning["];

    pub fn parse(block: &str) -> Option<Self> {
        let mut lines = block.lines().peekable();
        let command_line = lines.next_if(|line| Self::command_of(line).is_some());
        let mut example = Self {
            command_line: command_line.map(str::to_string),
            command: command_line.and_then(Self::command_of),
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
        let folder = ExampleFolder::holding(&self.files);
        let stages: Vec<Vec<String>> = match &self.command {
            Some(command) => vec![command.clone()],
            None => vec![vec![Self::CHECK.to_string()], vec![Self::BUILD.to_string()]],
        };
        stages
            .iter()
            .map(|command| Self::diagnostics_in(&folder.run(command, &self.input)))
            .find(|diagnostics| {
                diagnostics
                    .iter()
                    .any(|diagnostic| !diagnostic.code.is_empty())
            })
            .unwrap_or_default()
    }

    pub fn failure(&self) -> Option<String> {
        let folder = ExampleFolder::holding(&self.files);
        let command = self
            .command
            .clone()
            .unwrap_or_else(|| vec![Self::RUN.to_string()]);
        let output = folder.run(&command, &self.input);
        let reports_a_code = !Self::diagnostics_in(&output).is_empty();
        let failed = !output.status.success() || reports_a_code;
        failed.then(|| String::from_utf8_lossy(&output.stderr).into_owned())
    }

    pub fn layout(self) -> ExampleLayout {
        let mut layout = ExampleLayout::default();
        if let Some(command_line) = self.command_line {
            layout.keep_line(command_line);
        }
        if self.files.is_empty() {
            layout.lay_out_inputs(&self.input);
        }
        for file in self.files {
            layout.keep_line(format!("{}{}", Self::LABEL, file.path));
            layout.lay_out_file(file);
        }
        layout
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

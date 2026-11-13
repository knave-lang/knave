use std::process::Output;

use knave::language::Language;
use knave::reader::Reader;
use knave::source::SourceMap;

use super::example_folder::{ExampleFile, ExampleFolder};

#[derive(Default)]
pub struct ExampleLayout {
    parts: Vec<LayoutPart>,
}

enum LayoutPart {
    Kept(String),
    LaidOut(ExampleFile),
}

impl ExampleLayout {
    const SINGLE_FILE: &str = "example.knv";
    const INPUT_FILE: &str = "input";
    const FORMAT: &str = "fmt";
    const ERROR: &str = "error";
    const UNEXPLAINED_REFUSAL: &str = "`knave fmt` exits with a failure";

    pub fn of_single_file(block: &str) -> Self {
        let mut layout = Self::default();
        layout.lay_out_file(ExampleFile {
            path: Self::SINGLE_FILE.to_string(),
            text: block.to_string(),
        });
        layout
    }

    pub fn keep_line(&mut self, line: String) {
        self.parts.push(LayoutPart::Kept(line));
    }

    pub fn lay_out_file(&mut self, file: ExampleFile) {
        self.parts.push(LayoutPart::LaidOut(file));
    }

    pub fn lay_out_inputs(&mut self, input: &str) {
        for (index, text) in Self::inputs_of(input).into_iter().enumerate() {
            self.lay_out_file(ExampleFile {
                path: format!("{}-{index}.{}", Self::INPUT_FILE, Language::EXTENSION),
                text,
            });
        }
    }

    pub fn laid_out(&self) -> Result<String, String> {
        let folder = ExampleFolder::holding(self.files());
        let output = folder.run(&self.format_command(), "");
        if !output.status.success() {
            return Err(Self::refusal_of(&output));
        }
        Ok(self
            .parts
            .iter()
            .map(|part| part.text_in(&folder))
            .collect())
    }

    fn files(&self) -> impl Iterator<Item = &ExampleFile> {
        self.parts.iter().filter_map(LayoutPart::file)
    }

    fn format_command(&self) -> Vec<String> {
        let paths = self.files().map(|file| file.path.clone());
        std::iter::once(Self::FORMAT.to_string())
            .chain(paths)
            .collect()
    }

    fn refusal_of(output: &Output) -> String {
        String::from_utf8_lossy(&output.stderr)
            .lines()
            .find(|line| line.starts_with(Self::ERROR))
            .unwrap_or(Self::UNEXPLAINED_REFUSAL)
            .to_string()
    }

    fn inputs_of(input: &str) -> Vec<String> {
        let mut sources = SourceMap::default();
        let source = sources.add(Self::INPUT_FILE, input.to_string());
        match Reader::new(sources.file(source)).read() {
            Ok(forms) => forms
                .iter()
                .map(|form| format!("{}\n", &input[form.span().range()]))
                .collect(),
            Err(_) => vec![input.to_string()],
        }
    }
}

impl LayoutPart {
    fn file(&self) -> Option<&ExampleFile> {
        match self {
            LayoutPart::Kept(_) => None,
            LayoutPart::LaidOut(file) => Some(file),
        }
    }

    fn text_in(&self, folder: &ExampleFolder) -> String {
        match self {
            LayoutPart::Kept(line) => format!("{line}\n"),
            LayoutPart::LaidOut(file) => folder.text_of(file),
        }
    }
}

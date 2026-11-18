use std::fs;
use std::path::Path;

use super::example_folder::{ExampleFile, ExampleFolder};

pub struct SkillPage {
    folder_name: String,
    text: String,
}

#[derive(Default)]
struct SkillProgram {
    examples: Vec<SkillExample>,
    definitions: Vec<String>,
}

struct SkillExample {
    form: String,
    shown_value: String,
}

impl SkillPage {
    const FOLDER: &str = "skills/writing-knave";
    const FILE: &str = "SKILL.md";
    const NAME_FIELD: &str = "\nname: ";
    const EXAMPLE_FENCE: &str = "```knave\n";
    const CLOSING_FENCE: &str = "```\n";
    const PARAGRAPH_BREAK: &str = "\n\n";

    pub fn written() -> Self {
        let folder = Path::new(Self::FOLDER);
        let text = fs::read_to_string(folder.join(Self::FILE)).expect("the skill is readable");
        let folder_name = folder
            .file_name()
            .expect("the skill folder has a name")
            .to_string_lossy()
            .to_string();
        Self::new(folder_name, text)
    }

    pub fn new(folder_name: String, text: String) -> Self {
        Self { folder_name, text }
    }

    pub fn folder_name(&self) -> &str {
        &self.folder_name
    }

    pub fn declared_name(&self) -> Option<&str> {
        let (_, after_field) = self.text.split_once(Self::NAME_FIELD)?;
        after_field.lines().next()
    }

    pub fn value_failures(&self) -> Vec<String> {
        self.program().value_failures()
    }

    pub fn mislaid_program(&self) -> Option<String> {
        self.program().layout_failure()
    }

    fn program(&self) -> SkillProgram {
        let mut program = SkillProgram::default();
        for paragraph in self.code_paragraphs() {
            program.add(paragraph);
        }
        program
    }

    fn code_paragraphs(&self) -> impl Iterator<Item = &str> {
        self.text
            .split(Self::EXAMPLE_FENCE)
            .skip(1)
            .filter_map(|rest| rest.split(Self::CLOSING_FENCE).next())
            .flat_map(|block| block.trim_end().split(Self::PARAGRAPH_BREAK))
    }
}

impl SkillProgram {
    const PATH: &str = "skill.knv";
    const DOCSTRING_SUMMARY: &str = "Examples of the skill.";
    const DOCSTRING_INDENT: &str = "  ";
    const EVALUATE_COMMAND: &str = "eval";
    const FORMAT_COMMAND: &str = "fmt";

    fn add(&mut self, paragraph: &str) {
        match SkillExample::parse(paragraph) {
            Some(example) => self.examples.push(example),
            None => self.definitions.push(paragraph.to_string()),
        }
    }

    fn value_failures(&self) -> Vec<String> {
        let file = self.definitions_file();
        let folder = ExampleFolder::holding([&file]);
        self.examples
            .iter()
            .filter_map(|example| example.value_failure(&folder))
            .collect()
    }

    fn layout_failure(&self) -> Option<String> {
        let file = self.laid_out_file();
        let folder = ExampleFolder::holding([&file]);
        folder.run(
            &[Self::FORMAT_COMMAND.to_string(), Self::PATH.to_string()],
            "",
        );
        let laid_out = folder.text_of(&file);
        (laid_out != file.text).then(|| {
            format!("the skill's examples are not laid out as `knave fmt` lays out this program:\n\n{laid_out}")
        })
    }

    fn definitions_file(&self) -> ExampleFile {
        Self::file_of(self.definitions.iter().map(String::as_str))
    }

    fn laid_out_file(&self) -> ExampleFile {
        let examples: Vec<String> = self.examples.iter().map(SkillExample::paragraph).collect();
        let docstring = format!("{}\n\n{}", Self::DOCSTRING_SUMMARY, examples.join("\n\n"));
        let module = format!(
            "(module\n  \"{}\")",
            Self::indented(&Self::escaped(&docstring))
        );
        Self::file_of(
            std::iter::once(module.as_str()).chain(self.definitions.iter().map(String::as_str)),
        )
    }

    fn file_of<'f>(forms: impl Iterator<Item = &'f str>) -> ExampleFile {
        let forms: Vec<&str> = forms.collect();
        ExampleFile {
            path: Self::PATH.to_string(),
            text: format!("{}\n", forms.join("\n\n")),
        }
    }

    fn escaped(text: &str) -> String {
        text.replace('\\', "\\\\").replace('"', "\\\"")
    }

    fn indented(text: &str) -> String {
        text.lines()
            .map(|line| match line {
                "" => String::new(),
                line => format!("{}{line}", Self::DOCSTRING_INDENT),
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim_start()
            .to_string()
    }
}

impl SkillExample {
    const RESULT_MARK: &str = "\n=> ";

    fn parse(paragraph: &str) -> Option<Self> {
        let (form, shown_value) = paragraph.split_once(Self::RESULT_MARK)?;
        Some(Self {
            form: form.to_string(),
            shown_value: shown_value.to_string(),
        })
    }

    fn value_failure(&self, folder: &ExampleFolder) -> Option<String> {
        let command = [
            SkillProgram::EVALUATE_COMMAND.to_string(),
            SkillProgram::PATH.to_string(),
            self.form.clone(),
        ];
        let output = folder.run(&command, "");
        let printed = String::from_utf8_lossy(&output.stdout);
        let errors = String::from_utf8_lossy(&output.stderr);
        (printed.trim_end() != self.shown_value).then(|| {
            format!(
                "{} shows `=> {}` but `knave eval` prints:\n{printed}{errors}",
                self.form, self.shown_value
            )
        })
    }

    fn paragraph(&self) -> String {
        format!("{}{}{}", self.form, Self::RESULT_MARK, self.shown_value)
    }
}

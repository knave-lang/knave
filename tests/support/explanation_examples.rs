use std::path::PathBuf;

use knave::diagnostic::Explanation;

use super::Fixture;

pub struct ExplanationExamples {
    code: String,
    text: &'static str,
}

pub struct ExampleError {
    pub code: String,
    pub title: String,
}

impl ExplanationExamples {
    const ERRONEOUS_HEADING: &str = "## Erroneous example";
    const FIXES_HEADING: &str = "## Fixes";
    const EXAMPLE_FENCE: &str = "```lisp\n";
    const CLOSING_FENCE: &str = "```\n";
    const EXAMPLE_FILE: &str = "example.knv";

    pub fn written() -> Vec<Self> {
        Explanation::all()
            .filter_map(|explanation| {
                let text = explanation.text()?;
                let code = explanation.code().as_str().to_string();
                Some(Self { code, text })
            })
            .collect()
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn erroneous_errors(&self) -> Vec<Vec<ExampleError>> {
        self.examples_under(Self::ERRONEOUS_HEADING)
            .map(Self::errors_of)
            .collect()
    }

    pub fn fix_errors(&self) -> Vec<Vec<ExampleError>> {
        self.examples_under(Self::FIXES_HEADING)
            .map(Self::errors_of)
            .collect()
    }

    fn examples_under(&self, heading: &str) -> impl Iterator<Item = &'static str> {
        let section = self.section(heading);
        section
            .split(Self::EXAMPLE_FENCE)
            .skip(1)
            .filter_map(|rest| rest.split(Self::CLOSING_FENCE).next())
    }

    fn section(&self, heading: &str) -> &'static str {
        let after_heading = self.text.split_once(heading).map_or("", |(_, rest)| rest);
        after_heading.split("\n## ").next().unwrap_or(after_heading)
    }

    fn errors_of(example: &str) -> Vec<ExampleError> {
        let path = PathBuf::from(Fixture::FIXTURE_ROOT).join(Self::EXAMPLE_FILE);
        Fixture::of_text(&path, example.to_string()).check_errors()
    }
}

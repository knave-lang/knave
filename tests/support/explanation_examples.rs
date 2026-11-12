use std::path::PathBuf;
use std::thread;

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

    pub fn fix_count(&self) -> usize {
        self.examples_under(Self::FIXES_HEADING).count()
    }

    pub fn fix_failures(&self) -> Vec<String> {
        thread::scope(|scope| {
            let runs: Vec<_> = self
                .examples_under(Self::FIXES_HEADING)
                .enumerate()
                .map(|(index, fix)| scope.spawn(move || self.failure_of_fix(index, fix)))
                .collect();
            runs.into_iter()
                .filter_map(|run| run.join().expect("a fix run ends"))
                .collect()
        })
    }

    fn failure_of_fix(&self, index: usize, fix: &str) -> Option<String> {
        let file = format!("{}-fix-{index}.knv", self.code.to_lowercase());
        let path = PathBuf::from(Fixture::FIXTURE_ROOT).join(file);
        let failure = Fixture::of_text(&path, fix.to_string()).run_failure()?;
        Some(format!("fix {index} of {}:\n{failure}", self.code))
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

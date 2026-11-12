use std::path::PathBuf;
use std::thread;

use knave::diagnostic::Explanation;

use super::Fixture;

pub struct ExplanationExamples {
    code: String,
    text: &'static str,
}

pub struct ExampleDiagnostic {
    pub code: String,
    pub title: String,
}

impl ExplanationExamples {
    const ERRONEOUS_HEADING: &str = "## Erroneous example";
    const FIXES_HEADING: &str = "## Fixes";
    const EXAMPLE_FENCE: &str = "```lisp\n";
    const CLOSING_FENCE: &str = "```\n";
    const EXAMPLE_FILE: &str = "example.knv";
    const MAXIMUM_PARALLEL_FIX_BUILDS: usize = 4;

    pub fn new(code: &str, text: &'static str) -> Self {
        let code = code.to_string();
        Self { code, text }
    }

    pub fn written() -> Vec<Self> {
        Explanation::all()
            .filter_map(|explanation| {
                let text = explanation.text()?;
                Some(Self::new(explanation.code().as_str(), text))
            })
            .collect()
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn restated_titles(&self) -> Vec<String> {
        self.erroneous_diagnostics()
            .into_iter()
            .flatten()
            .filter(|diagnostic| diagnostic.code == self.code)
            .map(|diagnostic| diagnostic.title)
            .filter(|title| self.text.contains(title.as_str()))
            .collect()
    }

    fn erroneous_diagnostics(&self) -> Vec<Vec<ExampleDiagnostic>> {
        self.examples_under(Self::ERRONEOUS_HEADING)
            .map(|example| Self::fixture_of(example).reported_diagnostics())
            .collect()
    }

    pub fn erroneous_first_codes(&self) -> Vec<Option<String>> {
        self.examples_under(Self::ERRONEOUS_HEADING)
            .map(|example| Self::fixture_of(example).first_reported_code())
            .collect()
    }

    pub fn fix_count(&self) -> usize {
        self.examples_under(Self::FIXES_HEADING).count()
    }

    pub fn fix_failures(&self) -> Vec<String> {
        let fixes: Vec<(usize, &str)> = self
            .examples_under(Self::FIXES_HEADING)
            .enumerate()
            .collect();
        fixes
            .chunks(Self::MAXIMUM_PARALLEL_FIX_BUILDS)
            .flat_map(|batch| self.failures_of_batch(batch))
            .collect()
    }

    fn failures_of_batch(&self, batch: &[(usize, &str)]) -> Vec<String> {
        thread::scope(|scope| {
            let runs: Vec<_> = batch
                .iter()
                .map(|&(index, fix)| scope.spawn(move || self.failure_of_fix(index, fix)))
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

    fn fixture_of(example: &str) -> Fixture {
        let path = PathBuf::from(Fixture::FIXTURE_ROOT).join(Self::EXAMPLE_FILE);
        Fixture::of_text(&path, example.to_string())
    }
}

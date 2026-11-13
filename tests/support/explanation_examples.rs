use std::path::PathBuf;
use std::thread;

use knave::diagnostic::Explanation;

use super::Fixture;
use super::example_layout::ExampleLayout;
use super::project_example::ProjectExample;

pub struct ExplanationExamples {
    code: String,
    text: &'static str,
}

#[derive(Default)]
pub struct ExampleLayouts {
    pub mislaid: Vec<String>,
    pub skipped: Vec<String>,
}

pub struct ExampleDiagnostic {
    pub code: String,
    pub title: String,
}

impl ExplanationExamples {
    const ERRONEOUS_HEADING: &str = "## Erroneous example";
    const FIXES_HEADING: &str = "## Fixes";
    const EXAMPLE_FENCE: &str = "```knave\n";
    const CLOSING_FENCE: &str = "```\n";
    const EXAMPLE_FILE: &str = "example.knv";
    const MAXIMUM_PARALLEL_FIX_BUILDS: usize = 4;
    const ERRONEOUS_PLACE: &str = "erroneous example";
    const FIX_PLACE: &str = "fix";

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
            .map(Self::diagnostics_of)
            .collect()
    }

    pub fn erroneous_first_codes(&self) -> Vec<Option<String>> {
        self.examples_under(Self::ERRONEOUS_HEADING)
            .map(Self::first_code_of)
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

    pub fn layouts(&self) -> ExampleLayouts {
        let examples = self.placed_examples();
        let outcomes: Vec<(&String, &str, Result<String, String>)> = thread::scope(|scope| {
            let runs: Vec<_> = examples
                .iter()
                .map(|(place, example)| {
                    scope.spawn(move || (place, *example, Self::layout_of(example).laid_out()))
                })
                .collect();
            runs.into_iter()
                .map(|run| run.join().expect("a layout run ends"))
                .collect()
        });
        let mut layouts = ExampleLayouts::default();
        for (place, example, outcome) in outcomes {
            layouts.record(place, example, outcome);
        }
        layouts
    }

    fn placed_examples(&self) -> Vec<(String, &'static str)> {
        let erroneous = self.examples_placed(Self::ERRONEOUS_HEADING, Self::ERRONEOUS_PLACE);
        erroneous
            .chain(self.examples_placed(Self::FIXES_HEADING, Self::FIX_PLACE))
            .collect()
    }

    fn examples_placed<'e>(
        &'e self,
        heading: &'e str,
        place: &'e str,
    ) -> impl Iterator<Item = (String, &'static str)> + 'e {
        self.examples_under(heading)
            .enumerate()
            .map(move |(index, example)| (format!("{place} {index} of {}", self.code), example))
    }

    fn layout_of(example: &str) -> ExampleLayout {
        match ProjectExample::parse(example) {
            Some(project) => project.layout(),
            None => ExampleLayout::of_single_file(example),
        }
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
        let failure = match ProjectExample::parse(fix) {
            Some(project) => project.failure(),
            None => self.single_file_failure(index, fix),
        }?;
        Some(format!("fix {index} of {}:\n{failure}", self.code))
    }

    fn single_file_failure(&self, index: usize, fix: &str) -> Option<String> {
        let file = format!("{}-fix-{index}.knv", self.code.to_lowercase());
        let path = PathBuf::from(Fixture::FIXTURE_ROOT).join(file);
        Fixture::of_text(&path, fix.to_string()).run_failure()
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

    fn first_code_of(example: &str) -> Option<String> {
        match ProjectExample::parse(example) {
            Some(project) => project
                .reported_diagnostics()
                .into_iter()
                .map(|diagnostic| diagnostic.code)
                .find(|code| !code.is_empty()),
            None => Self::fixture_of(example).first_reported_code(),
        }
    }

    fn diagnostics_of(example: &str) -> Vec<ExampleDiagnostic> {
        match ProjectExample::parse(example) {
            Some(project) => project.reported_diagnostics(),
            None => Self::fixture_of(example).reported_diagnostics(),
        }
    }

    fn fixture_of(example: &str) -> Fixture {
        let path = PathBuf::from(Fixture::FIXTURE_ROOT).join(Self::EXAMPLE_FILE);
        Fixture::of_text(&path, example.to_string())
    }
}

impl ExampleLayouts {
    fn record(&mut self, place: &str, example: &str, outcome: Result<String, String>) {
        match outcome {
            Ok(laid_out) if laid_out == example => {}
            Ok(laid_out) => self.mislaid.push(format!(
                "{place} is not laid out as `knave fmt` lays it out. Replace the block with:\n\n\
                 {}{laid_out}{}",
                ExplanationExamples::EXAMPLE_FENCE,
                ExplanationExamples::CLOSING_FENCE
            )),
            Err(refusal) => self.skipped.push(format!("{place}: {refusal}")),
        }
    }
}

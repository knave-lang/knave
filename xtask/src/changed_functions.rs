use std::collections::BTreeSet;
use std::path::Path;

use crate::changed_lines::ChangedLines;
use crate::crap::ScoredFunction;
use crate::crap_forecast::{ChangedFunction, CrapForecast};
use crate::failure::TaskFailure;
use crate::functions::{SourceFunction, SourceTree};
use crate::gate::QualityGate;
use crate::lcov::LcovReport;
use crate::test_selection::{TestRun, TestSelection};
use crate::workspace::CargoWorkspace;

pub struct ChangedFunctions {
    functions: Vec<SourceFunction>,
}

impl ChangedFunctions {
    pub fn find(root: &Path, changed: &ChangedLines) -> Result<Self, TaskFailure> {
        let directories = CargoWorkspace::read(root)?.source_directories();
        let functions = SourceTree::new(root, directories).functions()?;
        Ok(Self::among(functions, changed))
    }

    fn among(functions: Vec<SourceFunction>, changed: &ChangedLines) -> Self {
        let functions = functions
            .into_iter()
            .filter(|function| changed.touches(&function.file, function.lines))
            .collect();
        Self { functions }
    }

    pub fn test_runs(&self, also: &[TestRun]) -> Vec<TestRun> {
        if self.functions.is_empty() {
            return Vec::new();
        }
        let files: BTreeSet<&str> = self
            .functions
            .iter()
            .map(|function| function.file.as_str())
            .collect();
        let mut runs = TestSelection::runs(files.into_iter());
        runs.extend(also.iter().cloned());
        runs.into_iter().collect()
    }

    pub fn forecast(self, report: &LcovReport) -> CrapForecast {
        let functions = self
            .functions
            .into_iter()
            .map(|function| Self::score(function, report))
            .collect();
        CrapForecast::new(QualityGate::MAXIMUM_CRAP, functions)
    }

    fn score(function: SourceFunction, report: &LcovReport) -> ChangedFunction {
        let missed = report.missed_within(&function.file, function.lines);
        let broader = TestSelection::broader_run(&function.file);
        let coverage = report.within(&function.file, function.lines);
        ChangedFunction {
            scored: ScoredFunction::new(function, coverage),
            missed,
            broader,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::ChangedFunctions;
    use crate::changed_lines::ChangedLines;
    use crate::checks::Check;
    use crate::functions::{LineSpan, SourceFunction};
    use crate::lcov::LcovReport;
    use crate::test_selection::TestRun;

    struct ChangeCase;

    impl ChangeCase {
        const DIFF: &str = "+++ b/src/emit/a.rs\n@@ -5 +5 @@\n";

        fn function(file: &str, first: usize, last: usize) -> SourceFunction {
            SourceFunction {
                file: file.to_string(),
                name: "area".to_string(),
                lines: LineSpan { first, last },
                complexity: 9,
            }
        }

        fn changed(functions: Vec<SourceFunction>) -> ChangedFunctions {
            ChangedFunctions::among(functions, &ChangedLines::parse(Self::DIFF))
        }

        fn runs(functions: Vec<SourceFunction>) -> Vec<String> {
            let also = [TestRun::parse("--test cli fmt")];
            Self::changed(functions)
                .test_runs(&also)
                .iter()
                .map(TestRun::to_string)
                .collect()
        }
    }

    #[test]
    fn a_function_the_diff_touches_selects_its_tests_and_the_extra_runs() {
        assert_eq!(
            ChangeCase::runs(vec![ChangeCase::function("src/emit/a.rs", 1, 9)]),
            [
                "--manifest-path Cargo.toml --lib emit::a",
                "--test cli fmt",
                "--test stages emit_fixtures"
            ]
        );
    }

    #[test]
    fn functions_the_diff_misses_run_nothing() {
        let untouched = vec![
            ChangeCase::function("src/emit/a.rs", 6, 9),
            ChangeCase::function("src/emit/b.rs", 1, 9),
        ];
        assert!(ChangeCase::runs(untouched).is_empty());
    }

    #[test]
    fn the_forecast_scores_each_changed_function() {
        let changed = ChangeCase::changed(vec![ChangeCase::function("src/emit/a.rs", 1, 9)]);
        let report = LcovReport::parse("SF:/work/src/emit/a.rs\nDA:5,1\n", Path::new("/work"));
        assert!(
            changed
                .forecast(&report)
                .summary()
                .starts_with("1 changed functions")
        );
    }
}

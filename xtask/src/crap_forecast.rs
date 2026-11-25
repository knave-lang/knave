use crate::changed_function::ChangedFunction;
use crate::checks::Check;
use crate::crap::ScoredFunction;
use crate::outlook::Outlook;

pub struct CrapForecast {
    maximum: f64,
    functions: Vec<ChangedFunction>,
}

impl CrapForecast {
    pub fn new(maximum: f64, functions: Vec<ChangedFunction>) -> Self {
        Self { maximum, functions }
    }

    fn with_outlook(&self, outlook: Outlook) -> impl Iterator<Item = &ChangedFunction> {
        self.functions
            .iter()
            .filter(move |function| function.outlook(self.maximum) == outlook)
    }

    fn failures(&self) -> String {
        let rows: String = self
            .with_outlook(Outlook::Fails)
            .map(ChangedFunction::row)
            .collect();
        if rows.is_empty() {
            return rows;
        }
        format!("{}{rows}", ScoredFunction::TABLE_HEADER)
    }

    fn unproven(&self) -> String {
        self.with_outlook(Outlook::Unproven)
            .map(ChangedFunction::unproven_advice)
            .collect()
    }
}

impl Check for CrapForecast {
    fn passes(&self) -> bool {
        self.with_outlook(Outlook::Passes).count() == self.functions.len()
    }

    fn summary(&self) -> String {
        format!(
            "{} changed functions scored against the CRAP limit of {}: {} over it in CI for certain, {} unproven",
            self.functions.len(),
            self.maximum,
            self.with_outlook(Outlook::Fails).count(),
            self.with_outlook(Outlook::Unproven).count()
        )
    }

    fn details(&self) -> String {
        format!("{}{}", self.failures(), self.unproven())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::CrapForecast;
    use crate::changed_function::ChangedFunction;
    use crate::checks::Check;
    use crate::crap::ScoredFunction;
    use crate::functions::{LineSpan, SourceFunction};
    use crate::lcov::LcovReport;
    use crate::test_selection::TestRun;

    struct ForecastCase;

    impl ForecastCase {
        const MAXIMUM: f64 = 8.0;
        const REPORT: &str = "SF:/work/src/a.rs\nDA:1,1\nDA:2,1\nDA:3,0\nDA:4,1\nend_of_record\n";
        const COVERED_REPORT: &str =
            "SF:/work/src/a.rs\nDA:1,1\nDA:2,1\nDA:3,1\nDA:4,1\nend_of_record\n";

        fn function(complexity: usize, broader: Option<&str>) -> ChangedFunction {
            Self::function_in(Self::REPORT, complexity, broader)
        }

        fn function_in(report: &str, complexity: usize, broader: Option<&str>) -> ChangedFunction {
            let report = LcovReport::parse(report, Path::new("/work"));
            let lines = LineSpan { first: 1, last: 4 };
            let function = SourceFunction {
                file: "src/a.rs".to_string(),
                name: "area".to_string(),
                lines,
                complexity,
            };
            ChangedFunction::new(
                ScoredFunction::new(function, report.within("src/a.rs", lines)),
                report.missed_within("src/a.rs", lines),
                broader.map(TestRun::parse),
            )
        }

        fn forecast(complexity: usize, broader: Option<&str>) -> CrapForecast {
            CrapForecast::new(Self::MAXIMUM, vec![Self::function(complexity, broader)])
        }
    }

    #[test]
    fn a_function_within_the_limit_passes() {
        let forecast = ForecastCase::forecast(7, None);
        assert!(forecast.passes());
        assert_eq!(forecast.details(), "");
    }

    #[test]
    fn a_fully_covered_function_at_the_limit_passes() {
        let function = ForecastCase::function_in(ForecastCase::COVERED_REPORT, 8, None);
        assert!(CrapForecast::new(ForecastCase::MAXIMUM, vec![function]).passes());
    }

    #[test]
    fn a_function_more_complex_than_the_limit_fails_for_certain() {
        let forecast = ForecastCase::forecast(9, None);
        assert!(!forecast.passes());
        assert_eq!(
            forecast.details(),
            "Split these functions or test their branches:\n     CRAP   CC  covered  function\n     10.3    9    75.0%  src/a.rs:1 area\n"
        );
    }

    #[test]
    fn a_partly_covered_function_over_the_limit_is_unproven() {
        let forecast = ForecastCase::forecast(8, Some("--test stages emit_fixtures"));
        assert!(!forecast.passes());
        assert_eq!(
            forecast.details(),
            "  src/a.rs:1 area scores 9.0 because the tests the forecast ran miss src/a.rs: 3. CI fails it unless another test covers those lines. If `cargo test --test stages emit_fixtures` covers them, dispatch the forecast again with `-f also='--test stages emit_fixtures'`; otherwise test them.\n"
        );
    }

    #[test]
    fn an_unmeasured_function_says_no_test_reached_it() {
        let function = ForecastCase::function_in("", 8, None);
        let details = CrapForecast::new(ForecastCase::MAXIMUM, vec![function]).details();
        assert!(
            details.starts_with("  src/a.rs:1 area scores 72.0 with no coverage record, so none of the tests the forecast ran reached it."),
            "{details}"
        );
    }

    #[test]
    fn an_unproven_function_without_a_broader_run_asks_for_its_test() {
        let details = ForecastCase::forecast(8, None).details();
        assert!(
            details.ends_with(
                "Name the test that covers them in the forecast's `also` input, or test them.\n"
            ),
            "{details}"
        );
    }

    #[test]
    fn the_summary_counts_each_outlook() {
        let functions = vec![
            ForecastCase::function(7, None),
            ForecastCase::function(8, None),
            ForecastCase::function(9, None),
        ];
        assert_eq!(
            CrapForecast::new(ForecastCase::MAXIMUM, functions).summary(),
            "3 changed functions scored against the CRAP limit of 8: 1 over it in CI for certain, 1 unproven"
        );
    }
}

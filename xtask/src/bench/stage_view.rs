use clap::ValueEnum;

use super::results::{ProgramResult, ResultSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum StageView {
    #[value(help = "Each stage apart from its allocator row")]
    Split,
    #[value(help = "Each stage with its allocator row folded into it")]
    Folded,
}

impl StageView {
    pub fn apply(self, results: ResultSet) -> ResultSet {
        match self {
            Self::Split => results,
            Self::Folded => ResultSet::new(results.programs().iter().map(Self::fold).collect()),
        }
    }

    fn fold(result: &ProgramResult) -> ProgramResult {
        ProgramResult {
            program: result.program.clone(),
            measurement: result
                .measurement
                .as_ref()
                .map(|measurement| measurement.with_allocator_rows_folded()),
            counters: result.counters.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::bench::comparison::Comparison;
    use crate::bench::measurement::Measurement;
    use crate::bench::results::Counters;

    struct StageViewFixture;

    impl StageViewFixture {
        const PROGRAM_INSTRUCTIONS: u64 = 1_000_000;

        fn emit(own: u64, allocator: u64) -> ResultSet {
            let stages = BTreeMap::from([
                ("emit".to_string(), own),
                ("emit allocator".to_string(), allocator),
            ]);
            ResultSet::new(vec![ProgramResult {
                program: "inventory".to_string(),
                measurement: Some(Measurement::new(stages, Self::PROGRAM_INSTRUCTIONS)),
                counters: Counters::new(),
            }])
        }

        fn report(view: StageView, before: ResultSet, after: ResultSet) -> String {
            let before = view.apply(before);
            let after = view.apply(after);
            Comparison::new(&before, &after, &BTreeSet::new()).full_report()
        }
    }

    #[test]
    fn the_folded_view_compares_each_stage_with_its_allocator_row() {
        let report = StageViewFixture::report(
            StageView::Folded,
            StageViewFixture::emit(100_000, 400_000),
            StageViewFixture::emit(300_000, 215_000),
        );
        assert!(report.contains("| inventory | emit | 500,000 | 515,000 | **+3.00%** |"));
        assert!(!report.contains("emit allocator"), "{report}");
    }

    #[test]
    fn the_split_view_compares_each_stage_apart_from_its_allocator_row() {
        let report = StageViewFixture::report(
            StageView::Split,
            StageViewFixture::emit(100_000, 400_000),
            StageViewFixture::emit(300_000, 215_000),
        );
        assert!(report.contains("| inventory | emit | 100,000 | 300,000 | **+200.00%** |"));
        assert!(report.contains("| inventory | emit allocator | 400,000 | 215,000 | -46.25% |"));
    }
}

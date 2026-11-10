use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::checks::Check;
use crate::failure::TaskFailure;

pub type ProductionLineCounts = BTreeMap<String, usize>;

pub struct ProductionLineBaseline {
    file: String,
    entries: ProductionLineCounts,
}

impl ProductionLineBaseline {
    pub fn read(root: &Path, file: &str) -> Result<Self, TaskFailure> {
        let path = root.join(file);
        let text =
            fs::read_to_string(&path).map_err(|error| TaskFailure::unreadable(&path, error))?;
        Self::parse(file, &text)
    }

    fn parse(file: &str, text: &str) -> Result<Self, TaskFailure> {
        let entries = text
            .lines()
            .map(|line| {
                Self::entry(line).ok_or_else(|| TaskFailure::malformed_baseline(file, line))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            file: file.to_string(),
            entries,
        })
    }

    fn entry(line: &str) -> Option<(String, usize)> {
        let (source, count) = line.split_once(' ')?;
        Some((source.to_string(), count.parse().ok()?))
    }
}

pub struct FileSizeLimit {
    maximum: usize,
    measured: ProductionLineCounts,
    baseline: ProductionLineBaseline,
}

impl FileSizeLimit {
    pub fn new(
        maximum: usize,
        measured: ProductionLineCounts,
        baseline: ProductionLineBaseline,
    ) -> Self {
        Self {
            maximum,
            measured,
            baseline,
        }
    }

    fn problems(&self) -> Vec<String> {
        let measured = self
            .measured
            .iter()
            .filter_map(|(source, &count)| self.problem(source, count));
        let vanished = self
            .baseline
            .entries
            .keys()
            .filter(|source| !self.measured.contains_key(*source))
            .map(|source| self.vanished(source));
        measured.chain(vanished).collect()
    }

    fn problem(&self, source: &str, count: usize) -> Option<String> {
        match self.baseline.entries.get(source) {
            Some(&listed) => self.listed_problem(source, count, listed),
            None => (count > self.maximum).then(|| self.oversized(source, count)),
        }
    }

    fn listed_problem(&self, source: &str, count: usize, listed: usize) -> Option<String> {
        if count <= self.maximum {
            return Some(self.within_limit(source, count));
        }
        match count.cmp(&listed) {
            Ordering::Greater => Some(self.grown(source, count, listed)),
            Ordering::Less => Some(self.shrunk(source, count, listed)),
            Ordering::Equal => None,
        }
    }

    fn oversized(&self, source: &str, count: usize) -> String {
        format!(
            "{source} has {count} production lines, over the limit of {}. Split it by responsibility.",
            self.maximum
        )
    }

    fn grown(&self, source: &str, count: usize, listed: usize) -> String {
        format!(
            "{source} has {count} production lines, over the limit of {} and above its baseline of {listed}. Split it by responsibility.",
            self.maximum
        )
    }

    fn shrunk(&self, source: &str, count: usize, listed: usize) -> String {
        format!(
            "{source} has {count} production lines, below its baseline of {listed}. Lower its entry in {} to {count}.",
            self.baseline.file
        )
    }

    fn within_limit(&self, source: &str, count: usize) -> String {
        format!(
            "{source} has {count} production lines, within the limit of {}. Remove its entry from {}.",
            self.maximum, self.baseline.file
        )
    }

    fn vanished(&self, source: &str) -> String {
        format!(
            "{source} is listed in {} but no longer exists. Remove its entry.",
            self.baseline.file
        )
    }
}

impl Check for FileSizeLimit {
    fn passes(&self) -> bool {
        self.problems().is_empty()
    }

    fn summary(&self) -> String {
        format!(
            "{} Rust files measured against the limit of {} production lines, {} of them listed in {}",
            self.measured.len(),
            self.maximum,
            self.baseline.entries.len(),
            self.baseline.file
        )
    }

    fn details(&self) -> String {
        self.problems()
            .iter()
            .map(|problem| format!("  {problem}\n"))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{FileSizeLimit, ProductionLineBaseline, ProductionLineCounts};
    use crate::checks::Check;

    struct Measurement;

    impl Measurement {
        const MAXIMUM: usize = 500;
        const BASELINE: &str = "xtask/baselines/production_lines.txt";

        fn limit(measured: &[(&str, usize)], baseline: &str) -> FileSizeLimit {
            let measured: ProductionLineCounts = measured
                .iter()
                .map(|&(source, count)| (source.to_string(), count))
                .collect();
            let baseline = ProductionLineBaseline::parse(Self::BASELINE, baseline)
                .expect("the baseline parses");
            FileSizeLimit::new(Self::MAXIMUM, measured, baseline)
        }

        fn details(measured: &[(&str, usize)], baseline: &str) -> String {
            let limit = Self::limit(measured, baseline);
            assert!(!limit.passes(), "the check passed");
            limit.details()
        }
    }

    #[test]
    fn an_unlisted_file_within_the_limit_passes() {
        assert!(Measurement::limit(&[("src/a.rs", 500)], "").passes());
    }

    #[test]
    fn an_unlisted_file_over_the_limit_fails() {
        assert_eq!(
            Measurement::details(&[("src/a.rs", 501)], ""),
            "  src/a.rs has 501 production lines, over the limit of 500. Split it by responsibility.\n"
        );
    }

    #[test]
    fn a_listed_file_at_its_baseline_passes() {
        assert!(Measurement::limit(&[("src/a.rs", 900)], "src/a.rs 900").passes());
    }

    #[test]
    fn a_listed_file_that_grew_fails() {
        assert_eq!(
            Measurement::details(&[("src/a.rs", 901)], "src/a.rs 900"),
            "  src/a.rs has 901 production lines, over the limit of 500 and above its baseline of 900. Split it by responsibility.\n"
        );
    }

    #[test]
    fn a_listed_file_that_shrank_must_lower_its_entry() {
        assert_eq!(
            Measurement::details(&[("src/a.rs", 899)], "src/a.rs 900"),
            "  src/a.rs has 899 production lines, below its baseline of 900. Lower its entry in xtask/baselines/production_lines.txt to 899.\n"
        );
    }

    #[test]
    fn a_listed_file_within_the_limit_must_leave_the_baseline() {
        assert_eq!(
            Measurement::details(&[("src/a.rs", 500)], "src/a.rs 900"),
            "  src/a.rs has 500 production lines, within the limit of 500. Remove its entry from xtask/baselines/production_lines.txt.\n"
        );
    }

    #[test]
    fn a_listed_file_that_no_longer_exists_must_leave_the_baseline() {
        assert_eq!(
            Measurement::details(&[], "src/gone.rs 900"),
            "  src/gone.rs is listed in xtask/baselines/production_lines.txt but no longer exists. Remove its entry.\n"
        );
    }

    #[test]
    fn the_summary_counts_the_files_and_the_baseline() {
        assert_eq!(
            Measurement::limit(&[("src/a.rs", 900), ("src/b.rs", 3)], "src/a.rs 900").summary(),
            "2 Rust files measured against the limit of 500 production lines, 1 of them listed in xtask/baselines/production_lines.txt"
        );
    }

    #[test]
    fn a_baseline_line_without_a_count_is_malformed() {
        let failure = ProductionLineBaseline::parse("baseline.txt", "src/a.rs many")
            .err()
            .expect("the baseline is malformed");
        assert_eq!(
            failure.to_string(),
            "baseline.txt has a malformed line \"src/a.rs many\": expected <file> <production lines>"
        );
    }

    #[test]
    fn a_baseline_line_without_a_separator_is_malformed() {
        assert!(ProductionLineBaseline::parse("baseline.txt", "src/a.rs").is_err());
    }
}

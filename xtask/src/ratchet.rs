use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;

use crate::checks::Check;
use crate::failure::TaskFailure;

pub type Counts = BTreeMap<String, usize>;

pub struct Unit {
    pub one: &'static str,
    pub many: &'static str,
}

impl Unit {
    fn count(&self, count: usize) -> String {
        let noun = if count == 1 { self.one } else { self.many };
        format!("{count} {noun}")
    }
}

pub struct Limit {
    pub baseline: &'static str,
    pub maximum: usize,
    pub subjects: &'static str,
    pub unit: Unit,
    pub advice: &'static str,
}

pub struct Baseline {
    file: String,
    entries: Counts,
}

impl Baseline {
    pub fn read(root: &Path, file: &str) -> Result<Self, TaskFailure> {
        let path = root.join(file);
        let text =
            fs::read_to_string(&path).map_err(|error| TaskFailure::unreadable(&path, error))?;
        Self::parse(file, &text)
    }

    fn parse(file: &str, text: &str) -> Result<Self, TaskFailure> {
        let mut entries = Counts::new();
        for line in text.lines() {
            let (subject, count) =
                Self::entry(line).ok_or_else(|| TaskFailure::malformed_baseline(file, line))?;
            if entries.insert(subject, count).is_some() {
                return Err(TaskFailure::repeated_baseline_entry(file, line));
            }
        }
        Ok(Self {
            file: file.to_string(),
            entries,
        })
    }

    fn entry(line: &str) -> Option<(String, usize)> {
        let (subject, count) = line.rsplit_once(' ')?;
        Some((subject.to_string(), count.parse().ok()?))
    }

    fn listed(&self, subject: &str) -> Option<usize> {
        self.entries.get(subject).copied()
    }

    fn vanished<'a>(&'a self, measured: &'a Counts) -> impl Iterator<Item = &'a str> {
        self.entries
            .keys()
            .filter(|subject| !measured.contains_key(*subject))
            .map(String::as_str)
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

impl fmt::Display for Baseline {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.file)
    }
}

enum Problem<'a> {
    Oversized {
        subject: &'a str,
        count: usize,
    },
    Grown {
        subject: &'a str,
        count: usize,
        listed: usize,
    },
    Shrunk {
        subject: &'a str,
        count: usize,
        listed: usize,
    },
    WithinLimit {
        subject: &'a str,
        count: usize,
    },
    Vanished {
        subject: &'a str,
    },
}

pub struct Ratchet {
    limit: Limit,
    measured: Counts,
    baseline: Baseline,
}

impl Ratchet {
    pub fn new(limit: Limit, measured: Counts, baseline: Baseline) -> Self {
        Self {
            limit,
            measured,
            baseline,
        }
    }

    fn problems(&self) -> Vec<Problem<'_>> {
        let measured = self
            .measured
            .iter()
            .filter_map(|(subject, &count)| self.problem(subject, count));
        let vanished = self
            .baseline
            .vanished(&self.measured)
            .map(|subject| Problem::Vanished { subject });
        measured.chain(vanished).collect()
    }

    fn problem<'a>(&self, subject: &'a str, count: usize) -> Option<Problem<'a>> {
        match self.baseline.listed(subject) {
            Some(listed) => self.listed_problem(subject, count, listed),
            None => (count > self.limit.maximum).then_some(Problem::Oversized { subject, count }),
        }
    }

    fn listed_problem<'a>(
        &self,
        subject: &'a str,
        count: usize,
        listed: usize,
    ) -> Option<Problem<'a>> {
        if count <= self.limit.maximum {
            return Some(Problem::WithinLimit { subject, count });
        }
        match count.cmp(&listed) {
            Ordering::Greater => Some(Problem::Grown {
                subject,
                count,
                listed,
            }),
            Ordering::Less => Some(Problem::Shrunk {
                subject,
                count,
                listed,
            }),
            Ordering::Equal => None,
        }
    }

    fn describe(&self, problem: &Problem) -> String {
        match *problem {
            Problem::Oversized { subject, count } => {
                format!("{}. {}", self.over_limit(subject, count), self.limit.advice)
            }
            Problem::Grown {
                subject,
                count,
                listed,
            } => format!(
                "{} and above its baseline of {listed}. {}",
                self.over_limit(subject, count),
                self.limit.advice
            ),
            Problem::Shrunk {
                subject,
                count,
                listed,
            } => format!(
                "{} and below its baseline of {listed}. Lower its entry in {} to {count}.",
                self.over_limit(subject, count),
                self.baseline
            ),
            Problem::WithinLimit { subject, count } => format!(
                "{subject} has {}, within the limit of {}. Remove its entry from {}.",
                self.limit.unit.count(count),
                self.limit.maximum,
                self.baseline
            ),
            Problem::Vanished { subject } => format!(
                "{subject} is listed in {} but no longer exists. Remove its entry.",
                self.baseline
            ),
        }
    }

    fn over_limit(&self, subject: &str, count: usize) -> String {
        format!(
            "{subject} has {}, over the limit of {}",
            self.limit.unit.count(count),
            self.limit.maximum
        )
    }
}

impl Check for Ratchet {
    fn passes(&self) -> bool {
        self.problems().is_empty()
    }

    fn summary(&self) -> String {
        format!(
            "{} {} measured against the limit of {}, {} of them listed in {}",
            self.measured.len(),
            self.limit.subjects,
            self.limit.unit.count(self.limit.maximum),
            self.baseline.len(),
            self.baseline
        )
    }

    fn details(&self) -> String {
        self.problems()
            .iter()
            .map(|problem| format!("  {}\n", self.describe(problem)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{Baseline, Counts, Limit, Ratchet, Unit};
    use crate::checks::Check;

    struct RatchetCase;

    impl RatchetCase {
        const BASELINE: &str = "xtask/baselines/production_lines.txt";

        fn ratchet(measured: &[(&str, usize)], baseline: &str) -> Ratchet {
            let measured: Counts = measured
                .iter()
                .map(|&(subject, count)| (subject.to_string(), count))
                .collect();
            let baseline = Baseline::parse(Self::BASELINE, baseline).expect("the baseline parses");
            Ratchet::new(Self::limit(), measured, baseline)
        }

        fn limit() -> Limit {
            Limit {
                baseline: Self::BASELINE,
                maximum: 500,
                subjects: "Rust files",
                unit: Unit {
                    one: "production line",
                    many: "production lines",
                },
                advice: "Split it by responsibility.",
            }
        }

        fn details(measured: &[(&str, usize)], baseline: &str) -> String {
            let ratchet = Self::ratchet(measured, baseline);
            assert!(!ratchet.passes(), "the check passed");
            ratchet.details()
        }

        fn baseline_failure(text: &str) -> String {
            match Baseline::parse("baseline.txt", text) {
                Ok(_) => panic!("the baseline parsed"),
                Err(failure) => failure.to_string(),
            }
        }
    }

    #[test]
    fn an_unlisted_subject_within_the_limit_passes() {
        assert!(RatchetCase::ratchet(&[("src/a.rs", 500)], "").passes());
    }

    #[test]
    fn an_unlisted_subject_over_the_limit_fails() {
        assert_eq!(
            RatchetCase::details(&[("src/a.rs", 501)], ""),
            "  src/a.rs has 501 production lines, over the limit of 500. Split it by responsibility.\n"
        );
    }

    #[test]
    fn a_listed_subject_at_its_baseline_passes() {
        assert!(RatchetCase::ratchet(&[("src/a.rs", 900)], "src/a.rs 900").passes());
    }

    #[test]
    fn a_listed_subject_that_grew_fails() {
        assert_eq!(
            RatchetCase::details(&[("src/a.rs", 901)], "src/a.rs 900"),
            "  src/a.rs has 901 production lines, over the limit of 500 and above its baseline of 900. Split it by responsibility.\n"
        );
    }

    #[test]
    fn a_listed_subject_that_shrank_must_lower_its_entry() {
        assert_eq!(
            RatchetCase::details(&[("src/a.rs", 899)], "src/a.rs 900"),
            "  src/a.rs has 899 production lines, over the limit of 500 and below its baseline of 900. Lower its entry in xtask/baselines/production_lines.txt to 899.\n"
        );
    }

    #[test]
    fn a_listed_subject_within_the_limit_must_leave_the_baseline() {
        assert_eq!(
            RatchetCase::details(&[("src/a.rs", 1)], "src/a.rs 900"),
            "  src/a.rs has 1 production line, within the limit of 500. Remove its entry from xtask/baselines/production_lines.txt.\n"
        );
    }

    #[test]
    fn a_listed_subject_at_the_limit_must_leave_the_baseline() {
        assert_eq!(
            RatchetCase::details(&[("src/a.rs", 500)], "src/a.rs 900"),
            "  src/a.rs has 500 production lines, within the limit of 500. Remove its entry from xtask/baselines/production_lines.txt.\n"
        );
    }

    #[test]
    fn a_listed_subject_that_no_longer_exists_must_leave_the_baseline() {
        assert_eq!(
            RatchetCase::details(&[], "src/gone.rs 900"),
            "  src/gone.rs is listed in xtask/baselines/production_lines.txt but no longer exists. Remove its entry.\n"
        );
    }

    #[test]
    fn the_summary_counts_the_subjects_and_the_baseline() {
        assert_eq!(
            RatchetCase::ratchet(&[("src/a.rs", 900), ("src/b.rs", 3)], "src/a.rs 900").summary(),
            "2 Rust files measured against the limit of 500 production lines, 1 of them listed in xtask/baselines/production_lines.txt"
        );
    }

    #[test]
    fn a_subject_may_hold_spaces() {
        let baseline = Baseline::parse("baseline.txt", "a b 3").expect("the baseline parses");
        assert_eq!(baseline.listed("a b"), Some(3));
    }

    #[test]
    fn a_baseline_line_without_a_count_is_malformed() {
        assert_eq!(
            RatchetCase::baseline_failure("src/a.rs many"),
            "baseline.txt has a malformed line \"src/a.rs many\": expected <subject> <count>"
        );
    }

    #[test]
    fn a_baseline_line_without_a_separator_is_malformed() {
        assert!(Baseline::parse("baseline.txt", "src/a.rs").is_err());
    }

    #[test]
    fn a_subject_listed_twice_is_rejected() {
        assert_eq!(
            RatchetCase::baseline_failure("src/a.rs 1\nsrc/a.rs 900"),
            "baseline.txt lists a subject twice, again in \"src/a.rs 900\""
        );
    }
}

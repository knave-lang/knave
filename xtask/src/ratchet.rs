use std::collections::{BTreeMap, BTreeSet};

use crate::checks::Check;
use crate::rise_approvals::RiseApprovals;

pub type Counts = BTreeMap<String, usize>;

#[derive(Clone, Copy)]
pub struct Unit {
    one: &'static str,
    many: &'static str,
}

impl Unit {
    pub const fn new(one: &'static str, many: &'static str) -> Self {
        Self { one, many }
    }

    fn count(&self, count: usize) -> String {
        let noun = if count == 1 { self.one } else { self.many };
        format!("{count} {noun}")
    }
}

#[derive(Clone, Copy)]
pub struct Limit {
    pub name: &'static str,
    pub maximum: usize,
    pub subjects: &'static str,
    pub unit: Unit,
    pub advice: &'static str,
}

pub struct Measurement {
    pub limit: Limit,
    pub counts: Counts,
}

pub struct Comparison {
    pub limit: Limit,
    pub head: Counts,
    pub base: Counts,
}

struct Rise<'a> {
    subject: &'a str,
    head: usize,
    base: Option<usize>,
}

pub struct Ratchet {
    limit: Limit,
    head: Counts,
    base: Counts,
    approved: BTreeSet<String>,
}

impl Ratchet {
    pub fn new(comparison: Comparison, approvals: &RiseApprovals) -> Self {
        Self {
            approved: approvals.subjects_of(comparison.limit.name),
            limit: comparison.limit,
            head: comparison.head,
            base: comparison.base,
        }
    }

    fn rises(&self) -> impl Iterator<Item = Rise<'_>> {
        self.head
            .iter()
            .filter(|&(_, &head)| head > self.limit.maximum)
            .map(|(subject, &head)| Rise {
                subject,
                head,
                base: self.base.get(subject).copied(),
            })
            .filter(|rise| rise.base.is_none_or(|base| rise.head > base))
    }

    fn refused(&self) -> impl Iterator<Item = Rise<'_>> {
        self.rises()
            .filter(|rise| !self.approved.contains(rise.subject))
    }

    fn over_limit(&self) -> usize {
        self.head
            .values()
            .filter(|&&count| count > self.limit.maximum)
            .count()
    }

    fn describe(&self, rise: &Rise) -> String {
        format!(
            "{} has {}, over the limit of {}, {}. {} If the project owner approves the rise, add this trailer to a commit of the branch:\n    {}",
            rise.subject,
            self.limit.unit.count(rise.head),
            self.limit.maximum,
            Self::base_clause(rise.base),
            self.limit.advice,
            RiseApprovals::trailer_line(self.limit.name, rise.subject)
        )
    }

    fn base_clause(base: Option<usize>) -> String {
        match base {
            Some(count) => format!("and {count} at the base"),
            None => "and did not exist at the base".to_string(),
        }
    }
}

impl Check for Ratchet {
    fn passes(&self) -> bool {
        self.refused().next().is_none()
    }

    fn summary(&self) -> String {
        format!(
            "{} {} measured against the limit of {}: {} over it, {} of them risen since the base without a `{}:` trailer",
            self.head.len(),
            self.limit.subjects,
            self.limit.unit.count(self.limit.maximum),
            self.over_limit(),
            self.refused().count(),
            RiseApprovals::TRAILER
        )
    }

    fn details(&self) -> String {
        self.refused()
            .map(|rise| format!("  {}\n", self.describe(&rise)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{Comparison, Counts, Limit, Ratchet, Unit};
    use crate::checks::Check;
    use crate::rise_approvals::RiseApprovals;

    struct RatchetCase;

    impl RatchetCase {
        const LIMIT: Limit = Limit {
            name: "production_lines",
            maximum: 500,
            subjects: "Rust files",
            unit: Unit::new("production line", "production lines"),
            advice: "Split it by responsibility.",
        };

        fn ratchet(head: &[(&str, usize)], base: &[(&str, usize)], log: &str) -> Ratchet {
            let comparison = Comparison {
                limit: Self::LIMIT,
                head: Self::counts(head),
                base: Self::counts(base),
            };
            Ratchet::new(comparison, &RiseApprovals::parse(log))
        }

        fn counts(entries: &[(&str, usize)]) -> Counts {
            entries
                .iter()
                .map(|&(subject, count)| (subject.to_string(), count))
                .collect()
        }
    }

    #[test]
    fn a_rise_over_the_limit_is_refused() {
        assert!(!RatchetCase::ratchet(&[("src/a.rs", 902)], &[("src/a.rs", 901)], "").passes());
    }

    #[test]
    fn a_rise_with_its_trailer_is_accepted() {
        let ratchet = RatchetCase::ratchet(
            &[("src/a.rs", 902)],
            &[("src/a.rs", 901)],
            "production_lines src/a.rs\n",
        );
        assert!(ratchet.passes());
    }

    #[test]
    fn a_fall_needs_nothing() {
        assert!(RatchetCase::ratchet(&[("src/a.rs", 901)], &[("src/a.rs", 902)], "").passes());
    }

    #[test]
    fn an_unchanged_count_over_the_limit_needs_nothing() {
        let counts = [("src/a.rs", 901)];
        assert!(RatchetCase::ratchet(&counts, &counts, "").passes());
    }

    #[test]
    fn a_new_subject_over_the_limit_is_measured_against_nothing() {
        let ratchet = RatchetCase::ratchet(&[("src/a.rs", 501)], &[], "");
        assert_eq!(
            ratchet.details(),
            "  src/a.rs has 501 production lines, over the limit of 500, and did not exist at the base. Split it by responsibility. If the project owner approves the rise, add this trailer to a commit of the branch:\n    Rise-Accepted: production_lines src/a.rs\n"
        );
    }

    #[test]
    fn a_deleted_subject_needs_nothing() {
        assert!(RatchetCase::ratchet(&[], &[("src/a.rs", 901)], "").passes());
    }

    #[test]
    fn a_subject_within_the_limit_needs_nothing_even_when_it_is_new() {
        assert!(RatchetCase::ratchet(&[("src/a.rs", 500)], &[], "").passes());
    }

    #[test]
    fn a_subject_that_crosses_the_limit_is_refused() {
        assert!(!RatchetCase::ratchet(&[("src/a.rs", 501)], &[("src/a.rs", 500)], "").passes());
    }

    #[test]
    fn a_trailer_of_another_limit_leaves_the_rise_refused() {
        let ratchet =
            RatchetCase::ratchet(&[("src/a.rs", 501)], &[], "methods_per_type src/a.rs\n");
        assert!(!ratchet.passes());
    }

    #[test]
    fn a_refusal_names_the_counts_the_limit_and_the_trailer() {
        let ratchet = RatchetCase::ratchet(&[("src/a.rs", 901)], &[("src/a.rs", 900)], "");
        assert_eq!(
            ratchet.details(),
            "  src/a.rs has 901 production lines, over the limit of 500, and 900 at the base. Split it by responsibility. If the project owner approves the rise, add this trailer to a commit of the branch:\n    Rise-Accepted: production_lines src/a.rs\n"
        );
    }

    #[test]
    fn the_summary_counts_the_subjects_over_the_limit_and_the_refused_rises() {
        let ratchet = RatchetCase::ratchet(
            &[("src/a.rs", 901), ("src/b.rs", 700), ("src/c.rs", 3)],
            &[("src/a.rs", 900), ("src/b.rs", 700)],
            "",
        );
        assert_eq!(
            ratchet.summary(),
            "3 Rust files measured against the limit of 500 production lines: 2 over it, 1 of them risen since the base without a `Rise-Accepted:` trailer"
        );
    }
}

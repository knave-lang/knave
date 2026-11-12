use std::collections::BTreeSet;

use crate::finding::Finding;
use crate::form_reducer::FormReducer;
use crate::harness::Harness;
use crate::known_classes::KnownClasses;
use crate::reduced_finding::ReducedFinding;
use crate::reduced_findings::ReducedFindings;
use crate::reducer::Reducer;
use crate::reported_signatures::ReportedSignatures;

pub struct ReductionPlan {
    reducer: Box<dyn Reducer>,
    classes: &'static KnownClasses,
    count: usize,
}

impl ReductionPlan {
    pub fn before_the_loan_rule(attempts: usize, count: usize) -> Self {
        Self::new(
            Box::new(FormReducer::new(attempts)),
            &KnownClasses::BEFORE_THE_LOAN_RULE,
            count,
        )
    }

    fn new(reducer: Box<dyn Reducer>, classes: &'static KnownClasses, count: usize) -> Self {
        Self {
            reducer,
            classes,
            count,
        }
    }

    pub fn reduce(
        &self,
        harness: &Harness,
        findings: &[Finding],
        reported: &ReportedSignatures,
    ) -> ReducedFindings {
        let mut ordered: Vec<&Finding> = findings.iter().collect();
        ordered.sort_by_key(|finding| finding.program.seed);
        let (known, outside): (Vec<&Finding>, Vec<&Finding>) = ordered
            .into_iter()
            .partition(|finding| finding.is_explained_by(self.classes));
        let mut seen = BTreeSet::new();
        let unreported = outside.into_iter().filter(|finding| {
            let signature = finding.signature();
            !reported.contain(&signature) && seen.insert(signature)
        });
        let outside_known_classes = self.reduced(harness, unreported.take(self.count));
        let left = self.count.saturating_sub(outside_known_classes.len());
        let known = self.reduced(harness, known.into_iter().take(left));
        ReducedFindings::new(outside_known_classes, known)
    }

    fn reduced<'a>(
        &self,
        harness: &Harness,
        findings: impl Iterator<Item = &'a Finding>,
    ) -> Vec<ReducedFinding> {
        findings
            .map(|finding| {
                ReducedFinding::new(
                    finding.clone(),
                    harness.reduced(finding, self.reducer.as_ref()),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::Duration;

    use super::ReductionPlan;
    use crate::finding::Finding;
    use crate::finding_fixture::FindingFixture;
    use crate::harness::Harness;
    use crate::knave_compiler::KnaveCompiler;
    use crate::reported_signatures::ReportedSignatures;
    use crate::scratch_directory::ScratchDirectory;
    use crate::seed::Seed;

    struct PlanFixture;

    impl PlanFixture {
        fn harness() -> Harness {
            Harness::new(
                KnaveCompiler::at(PathBuf::from("/nonexistent/knave")),
                1,
                Duration::from_secs(1),
            )
        }

        fn written(directory: &ScratchDirectory) -> Vec<String> {
            let mut names: Vec<String> = fs::read_dir(directory.path())
                .expect("the directory reads")
                .map(|entry| {
                    entry
                        .expect("an entry")
                        .file_name()
                        .to_string_lossy()
                        .to_string()
                })
                .collect();
            names.sort();
            names
        }
    }

    #[test]
    fn reduces_findings_outside_the_known_classes_first_then_known_ones_by_seed() {
        let findings = [
            FindingFixture::seeded(Seed::from(2), &["E0502"]),
            FindingFixture::seeded(Seed::from(1), &["E0716"]),
            FindingFixture::seeded(Seed::from(0), &["E0502"]),
        ];
        let reduced = ReductionPlan::before_the_loan_rule(10, 2).reduce(
            &PlanFixture::harness(),
            &findings,
            &ReportedSignatures::default(),
        );
        let directory = ScratchDirectory::create().expect("a scratch directory");
        reduced
            .record(directory.path())
            .expect("the reduced programs write");
        assert_eq!(
            PlanFixture::written(&directory),
            ["seed-0.reduced.knv", "seed-1.reduced.knv"]
        );
        let report = reduced
            .report_outside_known_classes()
            .expect("seed-1 is outside");
        assert!(report.contains("seed-1"));
        assert!(!report.contains("seed-0"));
    }

    #[test]
    fn spends_the_budget_on_signatures_not_yet_reported() {
        let posted = ["E0716", "E0597", "E0515"];
        let mut findings: Vec<Finding> = posted
            .iter()
            .enumerate()
            .map(|(seed, code)| FindingFixture::seeded(Seed::from(seed as u64), &[code]))
            .collect();
        findings.push(FindingFixture::seeded(Seed::from(3), &["E0716"]));
        findings.push(FindingFixture::seeded(Seed::from(4), &["E0373"]));
        let markers: String = findings[..posted.len()]
            .iter()
            .map(|finding| ReportedSignatures::marker(&finding.signature()) + "\n")
            .collect();
        let reported = ReportedSignatures::of_text(&markers);
        let reduced = ReductionPlan::before_the_loan_rule(10, 2).reduce(
            &PlanFixture::harness(),
            &findings,
            &reported,
        );
        let report = reduced
            .report_outside_known_classes()
            .expect("seed-4 is new");
        assert!(report.contains("seed-4"));
        assert!(!report.contains("seed-0") && !report.contains("seed-3"));
    }

    #[test]
    fn reduces_one_finding_per_signature_within_a_run() {
        let findings = [
            FindingFixture::seeded(Seed::from(0), &["E0716"]),
            FindingFixture::seeded(Seed::from(1), &["E0716"]),
        ];
        let reduced = ReductionPlan::before_the_loan_rule(10, 10).reduce(
            &PlanFixture::harness(),
            &findings,
            &ReportedSignatures::default(),
        );
        let report = reduced
            .report_outside_known_classes()
            .expect("seed-0 is new");
        assert!(report.contains("seed-0"));
        assert!(!report.contains("seed-1"));
    }
}

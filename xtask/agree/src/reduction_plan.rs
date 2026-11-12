use crate::finding::Finding;
use crate::form_reducer::FormReducer;
use crate::harness::Harness;
use crate::known_classes::KnownClasses;
use crate::reduced_finding::ReducedFinding;
use crate::reduced_findings::ReducedFindings;
use crate::reducer::Reducer;

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

    pub fn reduce(&self, harness: &Harness, findings: &[Finding]) -> ReducedFindings {
        let mut ordered: Vec<&Finding> = findings.iter().collect();
        ordered.sort_by_key(|finding| finding.program.seed);
        let (known, outside): (Vec<&Finding>, Vec<&Finding>) = ordered
            .into_iter()
            .partition(|finding| finding.is_explained_by(self.classes));
        let outside_known_classes = self.reduced(harness, outside.into_iter().take(self.count));
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
    use crate::finding_fixture::FindingFixture;
    use crate::harness::Harness;
    use crate::knave_compiler::KnaveCompiler;
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
        let reduced =
            ReductionPlan::before_the_loan_rule(10, 2).reduce(&PlanFixture::harness(), &findings);
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
}

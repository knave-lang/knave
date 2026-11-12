use std::io;

use crate::finding::Finding;
use crate::findings_directory::FindingsDirectory;
use crate::harness::Harness;
use crate::known_classes::KnownClasses;
use crate::reduction_plan::ReductionPlan;
use crate::subject_program::SubjectProgram;

pub struct ReducedFindings {
    reduced: Vec<(Finding, SubjectProgram)>,
    classes: &'static KnownClasses,
}

impl ReducedFindings {
    pub fn of(harness: &Harness, findings: &[Finding], plan: &ReductionPlan) -> Self {
        let mut ordered: Vec<&Finding> = findings.iter().collect();
        ordered.sort_by_key(|finding| plan.classes().explain(finding));
        Self {
            reduced: ordered
                .into_iter()
                .take(plan.count())
                .map(|finding| (finding.clone(), harness.reduced(finding, plan.reducer())))
                .collect(),
            classes: plan.classes(),
        }
    }

    pub fn record(&self, directory: &FindingsDirectory) -> io::Result<()> {
        self.reduced.iter().try_for_each(|(finding, program)| {
            directory.record_reduced(&finding.program.name, program)
        })
    }

    pub fn report_outside_known_classes(&self) -> Option<String> {
        let sections: Vec<String> = self
            .reduced
            .iter()
            .filter(|(finding, _)| !self.classes.explain(finding))
            .map(|(finding, program)| Self::section(finding, program))
            .collect();
        (!sections.is_empty()).then(|| sections.join("\n"))
    }

    fn section(finding: &Finding, program: &SubjectProgram) -> String {
        format!(
            "### `{}`\n\n{}\n\n```lisp\n{}```\n",
            finding.program.name,
            finding.line(),
            program.entry_text()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::ReducedFindings;
    use crate::finding_fixture::FindingFixture;
    use crate::known_classes::KnownClasses;

    #[test]
    fn reports_only_the_reduced_findings_outside_the_known_classes() {
        let reduced = ReducedFindings {
            reduced: vec![
                (
                    FindingFixture::citing("seed-1", &["E0502"]),
                    FindingFixture::program("seed-1", "(bad)\n"),
                ),
                (
                    FindingFixture::citing("seed-2", &["E0716"]),
                    FindingFixture::program("seed-2", "(worse)\n"),
                ),
            ],
            classes: &KnownClasses::BEFORE_THE_LOAN_RULE,
        };
        let report = reduced
            .report_outside_known_classes()
            .expect("one finding is new");
        assert!(report.contains("seed-2 Oracle: judged: E0716 in `main`"));
        assert!(report.contains("(worse)"));
        assert!(!report.contains("seed-1"));
    }

    #[test]
    fn reports_nothing_when_every_reduced_finding_is_known() {
        let reduced = ReducedFindings {
            reduced: vec![(
                FindingFixture::citing("seed-1", &["E0502"]),
                FindingFixture::program("seed-1", "(bad)\n"),
            )],
            classes: &KnownClasses::BEFORE_THE_LOAN_RULE,
        };
        assert_eq!(reduced.report_outside_known_classes(), None);
    }
}

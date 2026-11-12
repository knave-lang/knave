use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

use crate::reduced_finding::ReducedFinding;
use crate::reported_signatures::ReportedSignatures;

pub struct ReducedFindings {
    outside_known_classes: Vec<ReducedFinding>,
    known: Vec<ReducedFinding>,
}

impl ReducedFindings {
    pub fn new(outside_known_classes: Vec<ReducedFinding>, known: Vec<ReducedFinding>) -> Self {
        Self {
            outside_known_classes,
            known,
        }
    }

    pub fn record(&self, directory: &Path) -> io::Result<()> {
        self.outside_known_classes
            .iter()
            .chain(&self.known)
            .try_for_each(|reduced| reduced.record(directory))
    }

    pub fn write_report(&self, path: &Path, reported: &ReportedSignatures) -> io::Result<()> {
        match self.report_outside_known_classes(reported) {
            Some(report) => fs::write(path, report),
            None => Ok(()),
        }
    }

    pub fn report_outside_known_classes(&self, reported: &ReportedSignatures) -> Option<String> {
        let mut seen = BTreeSet::new();
        let sections: Vec<String> = self
            .outside_known_classes
            .iter()
            .filter(|reduced| !reported.contain(&reduced.signature()))
            .filter(|reduced| seen.insert(reduced.signature()))
            .map(ReducedFinding::section)
            .collect();
        (!sections.is_empty()).then(|| sections.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::ReducedFindings;
    use crate::finding_fixture::FindingFixture;
    use crate::reduced_finding::ReducedFinding;
    use crate::reported_signatures::ReportedSignatures;
    use crate::scratch_directory::ScratchDirectory;

    struct ReducedFixture;

    impl ReducedFixture {
        fn reduced(name: &str, code: &str, text: &str) -> ReducedFinding {
            ReducedFinding::new(
                FindingFixture::citing(name, &[code]),
                FindingFixture::program(name, text),
            )
        }
    }

    #[test]
    fn reports_only_the_reduced_findings_outside_the_known_classes() {
        let reduced = ReducedFindings::new(
            vec![ReducedFixture::reduced("seed-2", "E0716", "(worse)\n")],
            vec![ReducedFixture::reduced("seed-1", "E0502", "(bad)\n")],
        );
        let report = reduced
            .report_outside_known_classes(&ReportedSignatures::default())
            .expect("one finding is new");
        assert!(report.contains("seed-2 Oracle: judged: E0716 in `main`"));
        assert!(report.contains("(worse)"));
        assert!(!report.contains("seed-1"));
    }

    #[test]
    fn writes_a_report_file_only_when_a_finding_is_outside_the_known_classes() {
        let directory = ScratchDirectory::create().expect("a scratch directory");
        let path = directory.path().join("report.md");
        let quiet = ReducedFindings::new(
            Vec::new(),
            vec![ReducedFixture::reduced("seed-1", "E0502", "(bad)\n")],
        );
        quiet
            .write_report(&path, &ReportedSignatures::default())
            .expect("nothing to write");
        assert!(!path.exists());
        let loud = ReducedFindings::new(
            vec![ReducedFixture::reduced("seed-2", "E0716", "(worse)\n")],
            Vec::new(),
        );
        loud.write_report(&path, &ReportedSignatures::default())
            .expect("the report writes");
        assert!(
            fs::read_to_string(&path)
                .expect("the report reads")
                .contains("E0716")
        );
    }

    #[test]
    fn leaves_out_a_signature_already_reported_and_a_repeat_within_the_run() {
        let reduced = ReducedFindings::new(
            vec![
                ReducedFixture::reduced("seed-2", "E0716", "(worse)\n"),
                ReducedFixture::reduced("seed-3", "E0716", "(worse again)\n"),
                ReducedFixture::reduced("seed-4", "E0597", "(gone)\n"),
            ],
            Vec::new(),
        );
        let earlier = reduced
            .report_outside_known_classes(&ReportedSignatures::default())
            .expect("new findings");
        assert!(earlier.contains("seed-2") && !earlier.contains("seed-3"));
        let reported = ReportedSignatures::of_text(&earlier);
        assert_eq!(reduced.report_outside_known_classes(&reported), None);
    }

    #[test]
    fn reports_nothing_when_every_reduced_finding_is_known() {
        let reduced = ReducedFindings::new(
            Vec::new(),
            vec![ReducedFixture::reduced("seed-1", "E0502", "(bad)\n")],
        );
        assert_eq!(
            reduced.report_outside_known_classes(&ReportedSignatures::default()),
            None
        );
    }
}

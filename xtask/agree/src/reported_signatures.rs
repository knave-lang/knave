use std::collections::BTreeSet;

use crate::finding_signature::FindingSignature;

#[derive(Default)]
pub struct ReportedSignatures {
    signatures: BTreeSet<String>,
}

impl ReportedSignatures {
    const MARKER_START: &'static str = "<!-- agree-signature: ";
    const MARKER_END: &'static str = " -->";

    pub fn of_text(text: &str) -> Self {
        Self {
            signatures: text
                .lines()
                .filter_map(|line| line.trim().strip_prefix(Self::MARKER_START))
                .filter_map(|rest| rest.strip_suffix(Self::MARKER_END))
                .map(str::to_string)
                .collect(),
        }
    }

    pub fn marker(signature: &FindingSignature) -> String {
        format!("{}{signature}{}", Self::MARKER_START, Self::MARKER_END)
    }

    pub fn contain(&self, signature: &FindingSignature) -> bool {
        self.signatures.contains(&signature.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::ReportedSignatures;
    use crate::finding_fixture::FindingFixture;

    #[test]
    fn reads_back_the_markers_it_writes_from_any_text() {
        let posted = FindingFixture::citing("seed-1", &["E0716"]).signature();
        let other = FindingFixture::citing("seed-2", &["E0597"]).signature();
        let comments = format!(
            "Run: https://example.org/run/1\n\n### `seed-1`\n{}\n",
            ReportedSignatures::marker(&posted)
        );
        let reported = ReportedSignatures::of_text(&comments);
        assert!(reported.contain(&posted));
        assert!(!reported.contain(&other));
        assert!(!ReportedSignatures::default().contain(&posted));
    }
}

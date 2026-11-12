use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

use crate::finding::Finding;
use crate::finding_changes::FindingChanges;
use crate::finding_line::FindingLine;
use crate::seed::SeedRange;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FindingList {
    lines: BTreeSet<FindingLine>,
}

impl FindingList {
    pub fn of(findings: &[Finding]) -> Self {
        Self {
            lines: findings.iter().map(Finding::line).collect(),
        }
    }

    pub fn read(path: &Path) -> io::Result<Self> {
        Self::of_text(&fs::read_to_string(path)?)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    pub fn write(&self, path: &Path) -> io::Result<()> {
        fs::write(path, self.text())
    }

    pub fn text(&self) -> String {
        self.lines.iter().map(|line| format!("{line}\n")).collect()
    }

    pub fn within(&self, seeds: &SeedRange) -> Self {
        Self {
            lines: self
                .lines
                .iter()
                .filter(|line| line.seed().is_some_and(|seed| seeds.contains(seed)))
                .cloned()
                .collect(),
        }
    }

    pub fn changes_from(&self, expected: &FindingList) -> FindingChanges {
        FindingChanges::new(
            self.lines.difference(&expected.lines).cloned().collect(),
            expected.lines.difference(&self.lines).cloned().collect(),
        )
    }

    pub(crate) fn of_text(text: &str) -> Result<Self, String> {
        Ok(Self {
            lines: text
                .lines()
                .filter(|line| !line.is_empty())
                .map(str::parse)
                .collect::<Result<_, _>>()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::FindingList;
    use crate::seed::SeedRange;

    struct FindingListFixture;

    impl FindingListFixture {
        fn of(text: &str) -> FindingList {
            FindingList::of_text(text).expect("the fixture lists findings")
        }
    }

    #[test]
    fn refuses_a_line_without_a_judgement() {
        assert!(FindingList::of_text("lonely\n").is_err());
    }

    #[test]
    fn keeps_each_line_once_in_order_without_blank_lines() {
        let list = FindingListFixture::of("seed-2 a: x\n\nseed-1 b: y\nseed-2 a: x\n");
        assert_eq!(list.text(), "seed-1 b: y\nseed-2 a: x\n");
    }

    #[test]
    fn keeps_only_the_findings_of_a_seed_range() {
        let list = FindingListFixture::of("seed-1 a: x\nseed-25 a: y\nfixture a: z\n");
        let within = list.within(&SeedRange::new(0..20));
        assert_eq!(within.text(), "seed-1 a: x\n");
    }

    #[test]
    fn names_the_new_and_the_gone_findings() {
        let found = FindingListFixture::of("seed-1 a: x\nseed-3 a: z\n");
        let expected = FindingListFixture::of("seed-1 a: x\nseed-2 a: y\n");
        let changes = found.changes_from(&expected);
        assert!(!changes.is_empty());
        assert_eq!(
            changes.to_string(),
            "new finding: seed-3 a: z\nno longer found, remove it from the expected file: seed-2 a: y\n"
        );
        assert!(found.changes_from(&found).is_empty());
    }
}

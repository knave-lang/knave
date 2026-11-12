use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

use crate::finding::Finding;
use crate::finding_changes::FindingChanges;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FindingList {
    lines: BTreeSet<String>,
}

impl FindingList {
    pub fn of(findings: &[Finding]) -> Self {
        Self {
            lines: findings.iter().map(Finding::to_string).collect(),
        }
    }

    pub fn read(path: &Path) -> io::Result<Self> {
        Ok(Self::of_text(&fs::read_to_string(path)?))
    }

    pub fn of_text(text: &str) -> Self {
        Self {
            lines: text
                .lines()
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect(),
        }
    }

    pub fn write(&self, path: &Path) -> io::Result<()> {
        fs::write(path, self.text())
    }

    pub fn text(&self) -> String {
        self.lines.iter().map(|line| format!("{line}\n")).collect()
    }

    pub fn changes_from(&self, expected: &FindingList) -> FindingChanges {
        FindingChanges::new(
            self.lines.difference(&expected.lines).cloned().collect(),
            expected.lines.difference(&self.lines).cloned().collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::FindingList;

    #[test]
    fn keeps_each_line_once_in_order_without_blank_lines() {
        let list = FindingList::of_text("seed-2 a: x\n\nseed-1 b: y\nseed-2 a: x\n");
        assert_eq!(list.text(), "seed-1 b: y\nseed-2 a: x\n");
    }

    #[test]
    fn names_the_new_and_the_gone_findings() {
        let found = FindingList::of_text("seed-1 a: x\nseed-3 a: z\n");
        let expected = FindingList::of_text("seed-1 a: x\nseed-2 a: y\n");
        let changes = found.changes_from(&expected);
        assert!(!changes.is_empty());
        assert_eq!(
            changes.to_string(),
            "new finding: seed-3 a: z\nno longer found, remove it from the expected file: seed-2 a: y\n"
        );
        assert!(found.changes_from(&found).is_empty());
    }
}

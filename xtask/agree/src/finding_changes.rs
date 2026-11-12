use std::fmt;

use crate::finding_line::FindingLine;

#[derive(Debug, PartialEq, Eq)]
pub struct FindingChanges {
    new: Vec<FindingLine>,
    gone: Vec<FindingLine>,
}

impl FindingChanges {
    pub(crate) fn new(new: Vec<FindingLine>, gone: Vec<FindingLine>) -> Self {
        Self { new, gone }
    }

    pub fn is_empty(&self) -> bool {
        self.new.is_empty() && self.gone.is_empty()
    }
}

impl fmt::Display for FindingChanges {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in &self.new {
            writeln!(formatter, "new finding: {line}")?;
        }
        for line in &self.gone {
            writeln!(
                formatter,
                "no longer found, remove it from the expected file: {line}"
            )?;
        }
        Ok(())
    }
}

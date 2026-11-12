use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub struct FindingChanges {
    new: Vec<String>,
    gone: Vec<String>,
}

impl FindingChanges {
    pub fn new(new: Vec<String>, gone: Vec<String>) -> Self {
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

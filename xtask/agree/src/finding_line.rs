use std::fmt;
use std::str::FromStr;

use crate::seed::Seed;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FindingLine {
    program: String,
    judgement: String,
}

impl FindingLine {
    const SEPARATOR: char = ' ';

    pub fn new(program: String, judgement: String) -> Self {
        Self { program, judgement }
    }

    pub fn seed(&self) -> Option<Seed> {
        self.program.parse().ok()
    }
}

impl FromStr for FindingLine {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        let (program, judgement) = text
            .split_once(Self::SEPARATOR)
            .ok_or_else(|| format!("`{text}` names no program and judgement"))?;
        Ok(Self::new(program.to_string(), judgement.to_string()))
    }
}

impl fmt::Display for FindingLine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}{}{}",
            self.program,
            Self::SEPARATOR,
            self.judgement
        )
    }
}

#[cfg(test)]
mod tests {
    use super::FindingLine;
    use crate::seed::Seed;

    #[test]
    fn reads_back_what_it_writes_and_knows_its_seed() {
        let line: FindingLine = "seed-4 Oracle: rustc rejects E0502"
            .parse()
            .expect("a line");
        assert_eq!(line.to_string(), "seed-4 Oracle: rustc rejects E0502");
        assert_eq!(line.seed(), Some(Seed::from(4)));
        assert_eq!(
            "fixture Oracle: x"
                .parse::<FindingLine>()
                .map(|line| line.seed()),
            Ok(None)
        );
        assert!("lonely".parse::<FindingLine>().is_err());
    }
}

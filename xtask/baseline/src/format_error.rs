use std::error::Error;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub struct FormatError {
    message: String,
}

impl FormatError {
    pub fn malformed(name: &str, line: &str) -> Self {
        Self {
            message: format!("{name} has a malformed line {line:?}: expected <subject> <count>"),
        }
    }

    pub fn repeated(name: &str, line: &str) -> Self {
        Self {
            message: format!("{name} lists a subject twice, again in {line:?}"),
        }
    }
}

impl fmt::Display for FormatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for FormatError {}

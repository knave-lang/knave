#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl Severity {
    pub fn is_error(self) -> bool {
        self == Self::Error
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Severity;

    #[test]
    fn an_error_is_an_error() {
        assert!(Severity::Error.is_error());
    }

    #[test]
    fn a_warning_is_no_error() {
        assert!(!Severity::Warning.is_error());
    }
}

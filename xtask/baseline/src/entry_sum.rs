use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub struct EntrySum {
    pub subject: String,
    pub base: usize,
    pub ours: usize,
    pub theirs: usize,
    pub merged: usize,
}

impl EntrySum {
    const CLAMPED: &str = " (clamped)";
}

impl fmt::Display for EntrySum {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {} + {} - {} = {}",
            self.subject, self.ours, self.theirs, self.base, self.merged
        )?;
        if self.ours + self.theirs < self.base {
            formatter.write_str(Self::CLAMPED)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::EntrySum;

    #[test]
    fn a_sum_shows_its_arithmetic() {
        let sum = EntrySum {
            subject: "a".to_string(),
            base: 49,
            ours: 48,
            theirs: 48,
            merged: 47,
        };
        assert_eq!(sum.to_string(), "a: 48 + 48 - 49 = 47");
    }

    #[test]
    fn a_sum_below_zero_says_it_was_clamped() {
        let sum = EntrySum {
            subject: "a".to_string(),
            base: 1,
            ours: 0,
            theirs: 0,
            merged: 0,
        };
        assert_eq!(sum.to_string(), "a: 0 + 0 - 1 = 0 (clamped)");
    }
}

use std::collections::BTreeMap;

use crate::entry_sum::EntrySum;
use crate::format_error::FormatError;

pub type Counts = BTreeMap<String, usize>;

pub struct BaselineMerge {
    pub text: BaselineText,
    pub sums: Vec<EntrySum>,
}

pub struct BaselineText {
    name: String,
    lines: Vec<String>,
    entries: Counts,
}

impl BaselineText {
    const COUNT_SEPARATOR: char = ' ';

    pub fn parse(name: &str, text: &str) -> Result<Self, FormatError> {
        let mut entries = Counts::new();
        for line in text.lines().filter(|line| !Self::separates(line)) {
            let (subject, count) =
                Self::entry(line).ok_or_else(|| FormatError::malformed(name, line))?;
            if entries.insert(subject, count).is_some() {
                return Err(FormatError::repeated(name, line));
            }
        }
        Ok(Self {
            name: name.to_string(),
            lines: text.lines().map(String::from).collect(),
            entries,
        })
    }

    pub fn entries(&self) -> &Counts {
        &self.entries
    }

    pub fn crowded(&self) -> impl Iterator<Item = &str> {
        self.lines
            .iter()
            .zip(self.lines.iter().skip(1))
            .filter(|(above, line)| !Self::separates(above) && !Self::separates(line))
            .map(|(_, line)| line.as_str())
    }

    pub fn text(&self) -> String {
        self.lines.iter().map(|line| format!("{line}\n")).collect()
    }

    pub fn merged(&self, ours: &Self, theirs: &Self) -> BaselineMerge {
        let entries: Counts = ours
            .entries
            .keys()
            .chain(theirs.entries.keys())
            .filter_map(|subject| {
                let count = self.merged_count(ours, theirs, subject)?;
                Some((subject.clone(), count))
            })
            .collect();
        let layout = if ours.crowded().next().is_none() {
            ours
        } else {
            theirs
        };
        BaselineMerge {
            sums: self.sums(ours, theirs, &entries),
            text: Self {
                name: ours.name.clone(),
                lines: layout.relaid(&entries),
                entries,
            },
        }
    }

    fn sums(&self, ours: &Self, theirs: &Self, merged: &Counts) -> Vec<EntrySum> {
        self.entries
            .iter()
            .filter_map(|(subject, &base)| {
                let ours = *ours.entries.get(subject)?;
                let theirs = *theirs.entries.get(subject)?;
                let both_changed = ours != base && theirs != base;
                let merged = *merged.get(subject)?;
                both_changed.then(|| EntrySum {
                    subject: subject.clone(),
                    base,
                    ours,
                    theirs,
                    merged,
                })
            })
            .collect()
    }

    fn merged_count(&self, ours: &Self, theirs: &Self, subject: &str) -> Option<usize> {
        let ours = ours.entries.get(subject).copied();
        let theirs = theirs.entries.get(subject).copied();
        match self.entries.get(subject) {
            Some(base) => Some((ours? + theirs?).saturating_sub(*base)),
            None => ours.max(theirs),
        }
    }

    fn relaid(&self, entries: &Counts) -> Vec<String> {
        let mut lines: Vec<String> = self
            .lines
            .iter()
            .filter_map(|line| Self::relaid_line(line, entries))
            .collect();
        let added = entries
            .iter()
            .filter(|(subject, _)| !self.entries.contains_key(*subject));
        for (subject, count) in added {
            Self::insert(&mut lines, subject, *count);
        }
        lines
    }

    fn relaid_line(line: &str, entries: &Counts) -> Option<String> {
        match Self::entry(line) {
            Some((subject, _)) => entries
                .get(&subject)
                .map(|count| Self::line(&subject, *count)),
            None => Some(line.to_string()),
        }
    }

    fn insert(lines: &mut Vec<String>, subject: &str, count: usize) {
        let entry = Self::line(subject, count);
        let next_entry = lines.iter().position(|line| {
            Self::entry(line).is_some_and(|(listed, _)| listed.as_str() > subject)
        });
        match next_entry {
            Some(index) => {
                lines.splice(index..index, [entry, String::new()]);
            }
            None if lines.is_empty() => lines.push(entry),
            None => lines.extend([String::new(), entry]),
        }
    }

    fn line(subject: &str, count: usize) -> String {
        format!("{subject}{}{count}", Self::COUNT_SEPARATOR)
    }

    fn entry(line: &str) -> Option<(String, usize)> {
        let (subject, count) = line.rsplit_once(Self::COUNT_SEPARATOR)?;
        Some((subject.to_string(), count.parse().ok()?))
    }

    fn separates(line: &str) -> bool {
        line.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::BaselineText;
    use crate::format_error::FormatError;

    struct TextCase;

    impl TextCase {
        fn parse(text: &str) -> BaselineText {
            BaselineText::parse("baseline.txt", text).expect("the baseline parses")
        }

        fn failure(text: &str) -> FormatError {
            match BaselineText::parse("baseline.txt", text) {
                Ok(_) => panic!("the baseline parsed"),
                Err(failure) => failure,
            }
        }

        fn merged(base: &str, ours: &str, theirs: &str) -> String {
            let merge = Self::parse(base).merged(&Self::parse(ours), &Self::parse(theirs));
            merge.text.text()
        }

        fn crowded(text: &str) -> Vec<String> {
            Self::parse(text).crowded().map(String::from).collect()
        }
    }

    #[test]
    fn blank_lines_separate_entries() {
        let entries = TextCase::parse("a 1\n\n\nb 2\n").entries().clone();
        assert_eq!(
            entries.into_iter().collect::<Vec<_>>(),
            [("a".to_string(), 1), ("b".to_string(), 2)]
        );
    }

    #[test]
    fn a_subject_may_hold_spaces() {
        assert_eq!(TextCase::parse("a b 3").entries().get("a b"), Some(&3));
    }

    #[test]
    fn a_line_without_a_count_is_malformed() {
        assert_eq!(
            TextCase::failure("src/a.rs many").to_string(),
            "baseline.txt has a malformed line \"src/a.rs many\": expected <subject> <count>"
        );
    }

    #[test]
    fn a_line_without_a_separator_is_malformed() {
        assert_eq!(
            TextCase::failure("src/a.rs"),
            FormatError::malformed("baseline.txt", "src/a.rs")
        );
    }

    #[test]
    fn a_subject_listed_twice_is_rejected() {
        assert_eq!(
            TextCase::failure("src/a.rs 1\nsrc/a.rs 900").to_string(),
            "baseline.txt lists a subject twice, again in \"src/a.rs 900\""
        );
    }

    #[test]
    fn an_entry_right_below_another_is_crowded() {
        assert_eq!(TextCase::crowded("a 1\nb 2\n\nc 3\n"), ["b 2"]);
    }

    #[test]
    fn the_text_keeps_the_written_lines() {
        assert_eq!(TextCase::parse("a 1\n\n\nb 2").text(), "a 1\n\n\nb 2\n");
    }

    #[test]
    fn an_empty_baseline_has_no_text() {
        assert_eq!(TextCase::parse("").text(), "");
    }

    #[test]
    fn both_sides_lowering_an_entry_add_their_reductions() {
        assert_eq!(TextCase::merged("a 383", "a 380", "a 381"), "a 378\n");
    }

    #[test]
    fn reductions_past_zero_leave_zero() {
        assert_eq!(TextCase::merged("a 10", "a 1", "a 2"), "a 0\n");
    }

    #[test]
    fn a_side_that_keeps_an_entry_takes_the_other_side_change() {
        assert_eq!(
            TextCase::merged("a 10\n\nb 5", "a 9\n\nb 5", "a 10\n\nb 7"),
            "a 9\n\nb 7\n"
        );
    }

    #[test]
    fn a_merge_keeps_our_blank_lines() {
        assert_eq!(
            TextCase::merged("a 10\n\nb 5\n\nc 3", "a 10\n\n\nc 3", "a 10\n\nb 5\n\nc 2"),
            "a 10\n\n\nc 2\n"
        );
    }

    #[test]
    fn an_entry_either_side_removed_loses_only_its_line() {
        assert_eq!(TextCase::merged("a 10\n\nb 5", "a 9\n\n", "\n\nb 5"), "\n");
    }

    #[test]
    fn an_entry_only_theirs_added_goes_in_order_with_a_blank_line() {
        assert_eq!(
            TextCase::merged("a 1\n\nc 3", "a 1\n\nc 3", "a 1\n\nb 2\n\nc 3"),
            "a 1\n\nb 2\n\nc 3\n"
        );
    }

    #[test]
    fn an_entry_only_ours_added_stays() {
        assert_eq!(TextCase::merged("", "a 600", ""), "a 600\n");
    }

    #[test]
    fn an_entry_added_to_an_empty_baseline_stands_alone() {
        assert_eq!(TextCase::merged("", "", "a 600"), "a 600\n");
    }

    #[test]
    fn a_crowded_side_takes_the_layout_of_the_other() {
        assert_eq!(
            TextCase::merged("a 1\nb 2", "a 1\nb 2", "a 1\n\nb 1"),
            "a 1\n\nb 1\n"
        );
    }

    #[test]
    fn a_merge_lists_each_entry_both_sides_changed() {
        let base = TextCase::parse("a 49\n\nb 22\n\nc 5");
        let ours = TextCase::parse("a 48\n\nb 21\n\nc 5");
        let theirs = TextCase::parse("a 48\n\nb 22\n\nc 4");
        let sums: Vec<String> = base
            .merged(&ours, &theirs)
            .sums
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(sums, ["a: 48 + 48 - 49 = 47"]);
    }

    #[test]
    fn an_added_entry_after_every_other_goes_last() {
        assert_eq!(TextCase::merged("a 1", "a 1", "a 1\n\nz 9"), "a 1\n\nz 9\n");
    }

    #[test]
    fn an_added_entry_before_every_other_goes_first() {
        assert_eq!(TextCase::merged("b 1", "b 1", "a 9\n\nb 1"), "a 9\n\nb 1\n");
    }

    #[test]
    fn an_entry_both_sides_added_keeps_the_larger_count_for_the_ratchet_to_check() {
        assert_eq!(TextCase::merged("", "a 600", "a 700"), "a 700\n");
    }
}

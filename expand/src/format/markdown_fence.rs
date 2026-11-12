use crate::language::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkdownFence {
    mark: char,
    length: usize,
    holds_code: bool,
}

impl MarkdownFence {
    pub fn of_run(text: &str, mark: char) -> Self {
        let length = Self::run_length(text, mark);
        let info = text[length * mark.len_utf8()..].trim();
        Self {
            mark,
            length,
            holds_code: info.is_empty() || info == Language::NAME,
        }
    }

    pub fn length(self) -> usize {
        self.length
    }

    pub fn holds_code(self) -> bool {
        self.holds_code
    }

    pub fn is_closed_by(self, line: &str) -> bool {
        let text = line.trim_start();
        let length = Self::run_length(text, self.mark);
        length >= self.length && text[length * self.mark.len_utf8()..].trim().is_empty()
    }

    fn run_length(text: &str, mark: char) -> usize {
        text.chars()
            .take_while(|character| *character == mark)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::MarkdownFence;

    #[test]
    fn a_fence_without_a_language_holds_code() {
        assert!(MarkdownFence::of_run("```", '`').holds_code());
    }

    #[test]
    fn a_fence_that_names_knave_holds_code() {
        assert!(MarkdownFence::of_run("~~~ knave", '~').holds_code());
    }

    #[test]
    fn a_fence_that_names_another_language_holds_no_code() {
        assert!(!MarkdownFence::of_run("```rust", '`').holds_code());
    }

    #[test]
    fn a_fence_counts_its_run_of_marks() {
        assert_eq!(MarkdownFence::of_run("````x", '`').length(), 4);
    }

    #[test]
    fn a_fence_closes_on_a_run_of_its_mark_at_least_as_long() {
        let fence = MarkdownFence::of_run("````", '`');
        assert_eq!(
            (
                fence.is_closed_by("```"),
                fence.is_closed_by("~~~~"),
                fence.is_closed_by("````` "),
                fence.is_closed_by("```` x")
            ),
            (false, false, true, false)
        );
    }
}

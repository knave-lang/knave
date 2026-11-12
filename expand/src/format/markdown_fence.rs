use crate::language::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkdownFence {
    mark: char,
    length: usize,
    holds_code: bool,
}

impl MarkdownFence {
    const MARKS: [char; 2] = ['`', '~'];
    const MINIMUM_LENGTH: usize = 3;

    pub fn opening(line: &str) -> Option<Self> {
        let text = line.trim_start();
        let mark = Self::MARKS
            .into_iter()
            .find(|mark| text.starts_with(*mark))?;
        let length = Self::run_length(text, mark);
        let info = text[length * mark.len_utf8()..].trim();
        (length >= Self::MINIMUM_LENGTH).then_some(Self {
            mark,
            length,
            holds_code: info.is_empty() || info == Language::NAME,
        })
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
        assert!(MarkdownFence::opening("  ```").is_some_and(MarkdownFence::holds_code));
    }

    #[test]
    fn a_fence_that_names_knave_holds_code() {
        assert!(MarkdownFence::opening("~~~ knave").is_some_and(MarkdownFence::holds_code));
    }

    #[test]
    fn a_fence_that_names_another_language_holds_no_code() {
        assert!(MarkdownFence::opening("```rust").is_some_and(|fence| !fence.holds_code()));
    }

    #[test]
    fn two_marks_open_no_fence() {
        assert_eq!(MarkdownFence::opening("``"), None);
    }

    #[test]
    fn a_fence_closes_on_a_run_of_its_mark_at_least_as_long() {
        let fence = MarkdownFence::opening("````").expect("a fence");
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

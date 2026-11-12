#[derive(Default)]
pub struct PrintedText {
    text: String,
    started: bool,
}

impl PrintedText {
    pub fn push_line(&mut self, column: usize, rendered: &str) {
        if self.started {
            self.text.push('\n');
        }
        self.started = true;
        self.text.extend(std::iter::repeat_n(' ', column));
        self.text.push_str(rendered);
    }

    pub fn push_blank_line(&mut self) {
        self.text.push('\n');
    }

    pub fn is_started(&self) -> bool {
        self.started
    }

    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.text.split('\n')
    }

    pub fn into_string(self) -> String {
        self.text
    }
}

#[cfg(test)]
mod tests {
    use super::PrintedText;

    #[test]
    fn lines_follow_each_other_with_one_line_break() {
        let mut printed = PrintedText::default();
        printed.push_line(0, "(f 1)");
        printed.push_blank_line();
        printed.push_line(2, "(g 2)");
        assert_eq!(printed.into_string(), "(f 1)\n\n  (g 2)");
    }

    #[test]
    fn text_is_started_once_a_line_is_pushed() {
        let mut printed = PrintedText::default();
        let before = printed.is_started();
        printed.push_line(0, "; why");
        assert_eq!((before, printed.is_started()), (false, true));
    }

    #[test]
    fn lines_split_the_text_at_its_line_breaks() {
        let mut printed = PrintedText::default();
        printed.push_line(0, "(f\n  1)");
        assert_eq!(printed.lines().collect::<Vec<_>>(), ["(f", "  1)"]);
    }
}

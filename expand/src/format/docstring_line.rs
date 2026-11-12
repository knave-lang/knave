#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocstringLine(String);

impl From<String> for DocstringLine {
    fn from(text: String) -> Self {
        Self(text)
    }
}

impl DocstringLine {
    pub fn split(text: &str) -> Vec<Self> {
        text.split('\n')
            .map(|line| Self(line.to_string()))
            .collect()
    }

    pub fn least_indentation(lines: &[Self]) -> usize {
        lines
            .iter()
            .filter(|line| !line.is_blank())
            .map(Self::indentation)
            .min()
            .unwrap_or(0)
    }

    pub fn page_range(lines: &[Self], index: usize) -> std::ops::Range<usize> {
        let start: usize = lines[..index].iter().map(|line| line.0.len() + 1).sum();
        let line = &lines[index];
        start + line.indentation()..start + line.0.len()
    }

    pub fn text(&self) -> &str {
        &self.0
    }

    pub fn indentation(&self) -> usize {
        self.0.len() - self.0.trim_start().len()
    }

    pub fn is_blank(&self) -> bool {
        self.0.trim().is_empty()
    }

    pub fn text_after(&self, indent: usize) -> &str {
        self.0.get(indent..).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::DocstringLine;

    #[test]
    fn the_least_indentation_skips_blank_lines() {
        let lines = DocstringLine::split("    deep\n\n  shallow\n ");
        assert_eq!(DocstringLine::least_indentation(&lines), 2);
    }

    #[test]
    fn a_page_range_covers_a_line_after_its_indentation() {
        let lines = DocstringLine::split("first\n  (f 1)\nlast");
        assert_eq!(DocstringLine::page_range(&lines, 1), 8..13);
    }

    #[test]
    fn a_line_of_spaces_is_blank() {
        let lines = DocstringLine::split("   \nx");
        assert_eq!((lines[0].is_blank(), lines[1].is_blank()), (true, false));
    }

    #[test]
    fn the_text_after_an_indent_drops_that_many_bytes() {
        let lines = DocstringLine::split("    (f 1)\nx");
        assert_eq!(
            (lines[0].text_after(2), lines[1].text_after(4)),
            ("  (f 1)", "")
        );
    }
}

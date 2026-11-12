use std::ops::Range;

use super::code_lines::CodeLines;
use super::code_spacing::CodeSpacing;
use super::markdown_fence::MarkdownFence;

pub struct DocstringRegions<'l> {
    lines: &'l [String],
    fence_spacing: CodeSpacing,
    code: Vec<CodeLines>,
    paragraph_start: Option<usize>,
    fence: Option<(MarkdownFence, usize)>,
}

impl<'l> DocstringRegions<'l> {
    pub const ARROW: &'static str = "=>";
    const COMMENT: &'static str = ";";
    const SUMMARY_LINE: usize = 0;

    pub fn code_of(lines: &'l [String], fence_spacing: CodeSpacing) -> Vec<CodeLines> {
        let mut regions = Self {
            lines,
            fence_spacing,
            code: Vec::new(),
            paragraph_start: None,
            fence: None,
        };
        for index in 0..lines.len() {
            regions.visit(index);
        }
        regions.close_paragraph(lines.len());
        regions.code
    }

    pub fn is_arrow_line(line: &str) -> bool {
        let text = line.trim_start();
        text.strip_prefix(Self::ARROW)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
    }

    fn visit(&mut self, index: usize) {
        let line = &self.lines[index];
        match self.fence {
            Some(fence) => self.visit_fenced(index, fence),
            None => match MarkdownFence::opening(line) {
                Some(fence) => {
                    self.close_paragraph(index);
                    self.fence = Some((fence, index));
                }
                None if line.trim().is_empty() => self.close_paragraph(index),
                None => {
                    self.paragraph_start.get_or_insert(index);
                }
            },
        }
    }

    fn visit_fenced(&mut self, index: usize, (fence, opening): (MarkdownFence, usize)) {
        if !fence.is_closed_by(&self.lines[index]) {
            return;
        }
        self.fence = None;
        let inside = opening + 1..index;
        if fence.holds_code() && !inside.is_empty() {
            self.code.push(CodeLines {
                range: inside,
                spacing: self.fence_spacing,
            });
        }
    }

    fn close_paragraph(&mut self, end: usize) {
        let Some(start) = self.paragraph_start.take() else {
            return;
        };
        if self.is_example(start..end) {
            self.code.push(CodeLines {
                range: start..end,
                spacing: CodeSpacing::Written,
            });
        }
    }

    fn is_example(&self, paragraph: Range<usize>) -> bool {
        let lines = &self.lines[paragraph.clone()];
        paragraph.start != Self::SUMMARY_LINE
            && lines.iter().any(|line| Self::is_arrow_line(line))
            && !lines
                .iter()
                .any(|line| line.trim_start().starts_with(Self::COMMENT))
    }
}

#[cfg(test)]
mod tests {
    use super::DocstringRegions;
    use crate::format::code_spacing::CodeSpacing;

    struct RegionFixture;

    impl RegionFixture {
        fn code_of(text: &str) -> Vec<std::ops::Range<usize>> {
            let lines: Vec<String> = text.split('\n').map(str::to_string).collect();
            DocstringRegions::code_of(&lines, CodeSpacing::Separated)
                .into_iter()
                .map(|code| code.range)
                .collect()
        }

        fn spacings_of(text: &str) -> Vec<CodeSpacing> {
            let lines: Vec<String> = text.split('\n').map(str::to_string).collect();
            DocstringRegions::code_of(&lines, CodeSpacing::Separated)
                .into_iter()
                .map(|code| code.spacing)
                .collect()
        }
    }

    #[test]
    fn a_paragraph_with_an_arrow_is_code() {
        assert_eq!(
            RegionFixture::code_of("Doubles.\n\n(twice 3)\n=> 6\n\nMore prose."),
            vec![2..4]
        );
    }

    #[test]
    fn a_paragraph_without_an_arrow_is_prose() {
        assert!(RegionFixture::code_of("Doubles.\n\n(twice 3)\n(twice 4)").is_empty());
    }

    #[test]
    fn a_paragraph_with_a_comment_line_is_prose() {
        assert!(RegionFixture::code_of("Doubles.\n\n; why\n(twice 3)\n=> 6").is_empty());
    }

    #[test]
    fn an_arrow_in_the_summary_paragraph_is_prose() {
        assert!(RegionFixture::code_of("(twice 3)\n=> 6").is_empty());
    }

    #[test]
    fn the_inside_of_a_code_fence_is_code() {
        assert_eq!(
            RegionFixture::code_of("Doubles.\n\n```\n(twice 3)\n```\n\n```rust\nfn f() {}\n```"),
            vec![3..4]
        );
    }

    #[test]
    fn an_unclosed_fence_is_prose() {
        assert!(RegionFixture::code_of("Doubles.\n\n```\n(twice 3)\n=> 6").is_empty());
    }

    #[test]
    fn an_arrow_needs_a_space_or_the_end_of_its_line() {
        assert_eq!(
            (
                DocstringRegions::is_arrow_line("  => 6"),
                DocstringRegions::is_arrow_line("=>"),
                DocstringRegions::is_arrow_line("=>6")
            ),
            (true, true, false)
        );
    }

    #[test]
    fn an_example_keeps_its_written_spacing_and_a_fence_takes_the_fence_spacing() {
        assert_eq!(
            RegionFixture::spacings_of("Doubles.\n\n(twice 3)\n=> 6\n\n```\n(twice 3)\n```"),
            [CodeSpacing::Written, CodeSpacing::Separated]
        );
    }
}

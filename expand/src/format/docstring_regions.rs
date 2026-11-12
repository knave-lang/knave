use std::ops::Range;

use super::code_lines::CodeLines;
use super::code_spacing::CodeSpacing;
use super::docstring_syntax::DocstringSyntax;
use super::markdown_fence::MarkdownFence;
use crate::reader::Syntax;

pub struct DocstringRegions<'l> {
    syntax: &'l DocstringSyntax,
    lines: &'l [String],
    fence_spacing: CodeSpacing,
    code: Vec<CodeLines>,
    paragraph_start: Option<usize>,
    fence: Option<(MarkdownFence, usize)>,
}

impl<'l> DocstringRegions<'l> {
    const SUMMARY_LINE: usize = 0;

    pub fn code_of(
        syntax: &'l DocstringSyntax,
        lines: &'l [String],
        fence_spacing: CodeSpacing,
    ) -> Vec<CodeLines> {
        let mut regions = Self {
            syntax,
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

    fn visit(&mut self, index: usize) {
        let line = &self.lines[index];
        match self.fence {
            Some(fence) => self.visit_fenced(index, fence),
            None => match self.syntax.opening_fence(line) {
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
            && lines.iter().any(|line| self.syntax.is_arrow_line(line))
            && !lines
                .iter()
                .any(|line| line.trim_start().starts_with(Syntax::COMMENT))
    }
}

#[cfg(test)]
mod tests {
    use super::DocstringRegions;
    use crate::format::code_spacing::CodeSpacing;
    use crate::format::docstring_syntax::DocstringSyntaxFixture;

    struct RegionFixture;

    impl RegionFixture {
        fn code_of(text: &str) -> Vec<std::ops::Range<usize>> {
            let lines: Vec<String> = text.split('\n').map(str::to_string).collect();
            DocstringRegions::code_of(
                &DocstringSyntaxFixture::syntax(),
                &lines,
                CodeSpacing::Separated,
            )
            .into_iter()
            .map(|code| code.range)
            .collect()
        }

        fn spacings_of(text: &str) -> Vec<CodeSpacing> {
            let lines: Vec<String> = text.split('\n').map(str::to_string).collect();
            DocstringRegions::code_of(
                &DocstringSyntaxFixture::syntax(),
                &lines,
                CodeSpacing::Separated,
            )
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
    fn a_bare_arrow_is_prose() {
        assert!(RegionFixture::code_of("Doubles.\n\n(twice 3)\n=>").is_empty());
    }

    #[test]
    fn the_inside_of_a_tilde_fence_is_code() {
        assert_eq!(
            RegionFixture::code_of("Doubles.\n\n~~~\n(twice 3)\n~~~"),
            vec![3..4]
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

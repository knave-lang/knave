use std::ops::Range;

use super::docstring_line::DocstringLine;
use super::docstring_part::DocstringPart;
use super::docstring_syntax::DocstringSyntax;
use super::markdown_fence::MarkdownFence;
use crate::reader::Syntax;

pub struct DocstringRegions<'l> {
    syntax: &'l DocstringSyntax,
    lines: &'l [DocstringLine],
    code: Vec<DocstringPart>,
    paragraph_start: Option<usize>,
    fence: Option<(MarkdownFence, usize)>,
}

impl<'l> DocstringRegions<'l> {
    const SUMMARY_LINE: usize = 0;

    pub fn parts_of(syntax: &'l DocstringSyntax, lines: &'l [DocstringLine]) -> Vec<DocstringPart> {
        let mut regions = Self {
            syntax,
            lines,
            code: Vec::new(),
            paragraph_start: None,
            fence: None,
        };
        for index in 0..lines.len() {
            regions.visit(index);
        }
        regions.close_paragraph(lines.len());
        regions.with_prose()
    }

    fn visit(&mut self, index: usize) {
        let line = &self.lines[index];
        match self.fence {
            Some(fence) => self.visit_fenced(index, fence),
            None => match self.syntax.opening_fence(line.text()) {
                Some(fence) => {
                    self.close_paragraph(index);
                    self.fence = Some((fence, index));
                }
                None if line.is_blank() => self.close_paragraph(index),
                None => {
                    self.paragraph_start.get_or_insert(index);
                }
            },
        }
    }

    fn visit_fenced(&mut self, index: usize, (fence, opening): (MarkdownFence, usize)) {
        if !fence.is_closed_by(self.lines[index].text()) {
            return;
        }
        self.fence = None;
        let inside = opening + 1..index;
        if fence.holds_code() && !inside.is_empty() {
            self.code.push(DocstringPart::Fence(inside));
        }
    }

    fn close_paragraph(&mut self, end: usize) {
        let Some(start) = self.paragraph_start.take() else {
            return;
        };
        if self.is_example(start..end) {
            self.code.push(DocstringPart::Example(start..end));
        }
    }

    fn is_example(&self, paragraph: Range<usize>) -> bool {
        let lines = &self.lines[paragraph.clone()];
        paragraph.start != Self::SUMMARY_LINE
            && lines
                .iter()
                .any(|line| self.syntax.is_arrow_line(line.text()))
            && !lines
                .iter()
                .any(|line| line.text().trim_start().starts_with(Syntax::COMMENT))
    }

    fn with_prose(self) -> Vec<DocstringPart> {
        let mut parts = Vec::new();
        let mut covered = 0;
        for code in self.code {
            let range = code.range();
            if covered < range.start {
                parts.push(DocstringPart::Prose(covered..range.start));
            }
            covered = range.end;
            parts.push(code);
        }
        if covered < self.lines.len() {
            parts.push(DocstringPart::Prose(covered..self.lines.len()));
        }
        parts
    }
}

#[cfg(test)]
mod tests {
    use super::{DocstringLine, DocstringPart, DocstringRegions};
    use crate::format::docstring_syntax::DocstringSyntaxFixture;

    struct RegionFixture;

    impl RegionFixture {
        fn code_of(text: &str) -> Vec<std::ops::Range<usize>> {
            Self::parts_of(text)
                .into_iter()
                .filter(|part| !matches!(part, DocstringPart::Prose(_)))
                .map(|part| part.range())
                .collect()
        }

        fn parts_of(text: &str) -> Vec<DocstringPart> {
            let lines = DocstringLine::split(text);
            DocstringRegions::parts_of(&DocstringSyntaxFixture::syntax(), &lines)
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
    fn prose_examples_and_fences_cover_the_docstring_in_order() {
        assert_eq!(
            RegionFixture::parts_of("Doubles.\n\n(twice 3)\n=> 6\n\n```\n(twice 3)\n```"),
            [
                DocstringPart::Prose(0..2),
                DocstringPart::Example(2..4),
                DocstringPart::Prose(4..6),
                DocstringPart::Fence(6..7),
                DocstringPart::Prose(7..8)
            ]
        );
    }
}

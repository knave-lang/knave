use std::ops::Range;

use super::code_chunk::CodeChunk;
use super::code_layout::CodeLayout;
use super::docstring_line::DocstringLine;
use super::format_rules::FormatRules;
use super::region_layout::RegionLayout;
use super::written_gap::WrittenGap;
use crate::reader::{TokenKind, TokenScan};

pub struct CodeRegion<'s, 'l> {
    rules: FormatRules<'s>,
    lines: &'l [DocstringLine],
    indent: usize,
    layout: CodeLayout,
    laid_out: Vec<String>,
}

impl<'s, 'l> CodeRegion<'s, 'l> {
    pub fn new(rules: FormatRules<'s>, lines: &'l [DocstringLine], layout: CodeLayout) -> Self {
        Self {
            rules,
            lines,
            indent: DocstringLine::least_indentation(lines),
            layout,
            laid_out: Vec::new(),
        }
    }

    pub fn lay_out(mut self) -> RegionLayout {
        if self.holds_a_multi_line_string() {
            return RegionLayout::Prose;
        }
        match self.push_chunks() {
            Ok(()) if self.writes_a_fence_line() => RegionLayout::Prose,
            Ok(()) => RegionLayout::LaidOut(self.laid_out),
            Err(stop) => stop,
        }
    }

    fn holds_a_multi_line_string(&self) -> bool {
        let text = self
            .lines
            .iter()
            .map(DocstringLine::text)
            .collect::<Vec<_>>()
            .join("\n");
        TokenScan::new(&text)
            .any(|token| token.kind == TokenKind::String && text[token.range].contains('\n'))
    }

    fn writes_a_fence_line(&self) -> bool {
        self.laid_out
            .iter()
            .any(|line| self.rules.syntax.opening_fence(line).is_some())
    }

    fn push_chunks(&mut self) -> Result<(), RegionLayout> {
        let mut chunk_start = 0;
        let mut index = 0;
        while index < self.lines.len() {
            if self.rules.syntax.is_arrow_line(self.lines[index].text()) {
                self.push_chunk(chunk_start..index)?;
                index = self.push_result(index);
                chunk_start = index;
            } else {
                index += 1;
            }
        }
        self.push_chunk(chunk_start..self.lines.len())
    }

    fn push_chunk(&mut self, chunk: Range<usize>) -> Result<(), RegionLayout> {
        let Some(first) = chunk.clone().find(|line| !self.is_blank(*line)) else {
            return Ok(());
        };
        let written = if first > chunk.start {
            WrittenGap::BlankLine
        } else {
            WrittenGap::LineBreak
        };
        if !self.laid_out.is_empty() && self.layout.spacing.breaks_after_result(written) {
            self.laid_out.push(String::new());
        }
        let text = self.lines[first..chunk.end]
            .iter()
            .map(|line| line.text_after(self.indent))
            .collect::<Vec<_>>()
            .join("\n");
        match CodeChunk::new(self.rules.shapes, self.indent, self.layout).lay_out(text.trim_end()) {
            RegionLayout::LaidOut(lines) if self.writes_an_arrow_line(&lines) => {
                Err(RegionLayout::Prose)
            }
            RegionLayout::LaidOut(lines) => {
                self.laid_out.extend(lines);
                Ok(())
            }
            RegionLayout::Unreadable(line) => Err(RegionLayout::Unreadable(first + line)),
            RegionLayout::Prose => Err(RegionLayout::Prose),
        }
    }

    fn writes_an_arrow_line(&self, lines: &[String]) -> bool {
        lines
            .iter()
            .any(|line| self.rules.syntax.is_arrow_line(line))
    }

    fn push_result(&mut self, arrow: usize) -> usize {
        self.laid_out.push(self.lines[arrow].text().to_string());
        let mut index = arrow + 1;
        while index < self.lines.len() && self.continues_result(index) {
            self.laid_out.push(self.lines[index].text().to_string());
            index += 1;
        }
        index
    }

    fn continues_result(&self, index: usize) -> bool {
        let line = &self.lines[index];
        !line.is_blank()
            && line.indentation() > self.indent
            && !self.rules.syntax.is_arrow_line(line.text())
    }

    fn is_blank(&self, index: usize) -> bool {
        self.lines[index].is_blank()
    }
}

#[cfg(test)]
mod tests {
    use super::{CodeRegion, DocstringLine, RegionLayout};
    use crate::format::code_layout::CodeLayout;
    use crate::format::code_spacing::CodeSpacing;
    use crate::format::docstring_syntax::{DocstringSyntax, DocstringSyntaxFixture};
    use crate::format::format_rules::FormatRules;
    use crate::format::shapes::ShapeCatalog;

    struct RegionFixture {
        shapes: ShapeCatalog,
        syntax: DocstringSyntax,
    }

    impl RegionFixture {
        fn new() -> Self {
            Self {
                shapes: ShapeCatalog::default(),
                syntax: DocstringSyntaxFixture::syntax(),
            }
        }

        fn lay_out(&self, text: &str) -> RegionLayout {
            self.lay_out_spaced(text, CodeSpacing::Written)
        }

        fn lay_out_spaced(&self, text: &str, spacing: CodeSpacing) -> RegionLayout {
            let lines = DocstringLine::split(text);
            let layout = CodeLayout { column: 2, spacing };
            let rules = FormatRules {
                shapes: &self.shapes,
                syntax: &self.syntax,
            };
            CodeRegion::new(rules, &lines, layout).lay_out()
        }
    }

    #[test]
    fn a_result_line_stays_as_written_under_its_code() {
        let fixture = RegionFixture::new();
        assert_eq!(
            fixture.lay_out("  (f   1)\n\n  =>   (some  1)"),
            RegionLayout::LaidOut(vec!["  (f 1)".to_string(), "  =>   (some  1)".to_string()])
        );
    }

    #[test]
    fn a_deeper_line_after_a_result_continues_it() {
        let fixture = RegionFixture::new();
        assert_eq!(
            fixture.lay_out("(f 1)\n=> (vec 1\n        2)\n(g  2)\n=> 3"),
            RegionLayout::LaidOut(vec![
                "(f 1)".to_string(),
                "=> (vec 1".to_string(),
                "        2)".to_string(),
                "(g 2)".to_string(),
                "=> 3".to_string()
            ])
        );
    }

    #[test]
    fn a_blank_line_after_a_result_stays() {
        let fixture = RegionFixture::new();
        assert_eq!(
            fixture.lay_out("(f 1)\n=> 1\n\n(g 2)"),
            RegionLayout::LaidOut(vec![
                "(f 1)".to_string(),
                "=> 1".to_string(),
                String::new(),
                "(g 2)".to_string()
            ])
        );
    }

    #[test]
    fn code_that_does_not_read_names_its_first_line() {
        let fixture = RegionFixture::new();
        assert_eq!(
            fixture.lay_out("(f 1)\n=> 1\n\n(g 2\n=> 2"),
            RegionLayout::Unreadable(3)
        );
    }

    #[test]
    fn an_example_whose_form_shares_its_line_is_prose() {
        let fixture = RegionFixture::new();
        assert_eq!(
            fixture.lay_out("(see below) and more\n=> 7"),
            RegionLayout::Prose
        );
    }

    #[test]
    fn separated_spacing_puts_a_blank_line_after_a_result() {
        let fixture = RegionFixture::new();
        assert_eq!(
            fixture.lay_out_spaced("(f 1)\n=> 1\n(g 2)\n\n=> 2", CodeSpacing::Separated),
            RegionLayout::LaidOut(vec![
                "(f 1)".to_string(),
                "=> 1".to_string(),
                String::new(),
                "(g 2)".to_string(),
                "=> 2".to_string()
            ])
        );
    }

    #[test]
    fn code_that_holds_a_multi_line_string_is_prose() {
        let fixture = RegionFixture::new();
        assert_eq!(
            fixture.lay_out("  (count   \"first\n  second\")\n  => 2"),
            RegionLayout::Prose
        );
    }

    #[test]
    fn code_that_would_start_a_line_with_an_arrow_is_prose() {
        let fixture = RegionFixture::new();
        let (first, second) = ("a".repeat(36), "b".repeat(36));
        assert_eq!(
            fixture.lay_out(&format!("(arrows ({first} 1) => {second} (c 2))\n=> 1")),
            RegionLayout::Prose
        );
    }

    #[test]
    fn code_that_writes_a_fence_line_is_prose() {
        let fixture = RegionFixture::new();
        assert_eq!(
            fixture.lay_out_spaced("~~~\n(exit 1)\n=> 7", CodeSpacing::Separated),
            RegionLayout::Prose
        );
    }
}

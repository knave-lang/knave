use std::ops::Range;

use super::as_written_checks::AsWrittenChecks;
use super::code_chunk::CodeChunk;
use super::code_layout::CodeLayout;
use super::docstring_line::DocstringLine;
use super::format_rules::FormatRules;
use super::region_layout::RegionLayout;
use super::unreadable_cause::UnreadableCause;
use super::unreadable_code::UnreadableCode;
use super::written_gap::WrittenGap;
use crate::reader::{TokenKind, TokenScan};

pub struct CodeRegion<'s, 'l> {
    rules: FormatRules<'s>,
    checks: AsWrittenChecks<'s>,
    lines: &'l [DocstringLine],
    indent: usize,
    layout: CodeLayout,
    laid_out: Vec<String>,
}

impl<'s, 'l> CodeRegion<'s, 'l> {
    pub fn new(rules: FormatRules<'s>, lines: &'l [DocstringLine], layout: CodeLayout) -> Self {
        Self {
            rules,
            checks: AsWrittenChecks::new(rules.syntax),
            lines,
            indent: DocstringLine::least_indentation(lines),
            layout,
            laid_out: Vec::new(),
        }
    }

    pub fn lay_out(mut self) -> RegionLayout {
        if self.checks.holds_a_multi_line_string(self.lines) {
            return RegionLayout::Prose;
        }
        match self.push_chunks() {
            Ok(()) if self.checks.writes_a_fence_line(&self.laid_out) => RegionLayout::Prose,
            Ok(()) => RegionLayout::LaidOut(self.laid_out),
            Err(stop) => stop,
        }
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
            RegionLayout::LaidOut(lines) if self.checks.writes_an_arrow_line(&lines) => {
                Err(RegionLayout::Prose)
            }
            RegionLayout::LaidOut(lines) => {
                self.laid_out.extend(lines);
                Ok(())
            }
            RegionLayout::Unreadable(code) => Err(RegionLayout::Unreadable(UnreadableCode {
                line: first + code.line,
                cause: self.cause_of(chunk.start, &text, code.cause),
            })),
            RegionLayout::Prose => Err(RegionLayout::Prose),
        }
    }

    fn cause_of(&self, chunk_start: usize, text: &str, cause: UnreadableCause) -> UnreadableCause {
        let follows_a_result = chunk_start > 0 && !self.is_blank(chunk_start);
        if follows_a_result && Self::closes_more_than_it_opens(text) {
            UnreadableCause::ShallowResult
        } else {
            cause
        }
    }

    fn closes_more_than_it_opens(text: &str) -> bool {
        let balance = TokenScan::new(text).fold(0, |balance: isize, token| match token.kind {
            TokenKind::Open => balance + 1,
            TokenKind::Close => balance - 1,
            _ => balance,
        });
        balance < 0
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
    use super::{CodeRegion, DocstringLine, RegionLayout, UnreadableCause, UnreadableCode};
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
            RegionLayout::Unreadable(UnreadableCode {
                line: 3,
                cause: UnreadableCause::Code
            })
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
    fn a_result_line_no_deeper_than_its_code_is_a_shallow_result() {
        let fixture = RegionFixture::new();
        assert_eq!(
            fixture.lay_out("(pairs   1)\n=> (vec 1\n2)"),
            RegionLayout::Unreadable(UnreadableCode {
                line: 2,
                cause: UnreadableCause::ShallowResult
            })
        );
    }

    #[test]
    fn code_after_a_blank_line_that_does_not_read_is_unreadable_code() {
        let fixture = RegionFixture::new();
        assert_eq!(
            fixture.lay_out("(f 1)\n=> 1\n\n2)\n=> 2"),
            RegionLayout::Unreadable(UnreadableCode {
                line: 3,
                cause: UnreadableCause::Code
            })
        );
    }

    #[test]
    fn unreadable_code_after_a_result_that_opens_as_many_lists_as_it_closes_is_unreadable_code() {
        let fixture = RegionFixture::new();
        assert_eq!(
            fixture.lay_out("(f 1)\n=> 1\n(g 2)) (h\n=> 2"),
            RegionLayout::Unreadable(UnreadableCode {
                line: 2,
                cause: UnreadableCause::Code
            })
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

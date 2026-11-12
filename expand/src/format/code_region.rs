use std::ops::Range;

use super::chunk_layout::ChunkLayout;
use super::code_chunk::CodeChunk;
use super::code_layout::CodeLayout;
use super::format_rules::FormatRules;
use super::region_layout::RegionLayout;
use super::written_gap::WrittenGap;

pub struct CodeRegion<'s, 'l> {
    rules: FormatRules<'s>,
    lines: &'l [String],
    indent: usize,
    layout: CodeLayout,
    laid_out: Vec<String>,
}

impl<'s, 'l> CodeRegion<'s, 'l> {
    pub fn new(rules: FormatRules<'s>, lines: &'l [String], layout: CodeLayout) -> Self {
        Self {
            rules,
            lines,
            indent: Self::least_indentation(lines),
            layout,
            laid_out: Vec::new(),
        }
    }

    pub fn least_indentation(lines: &[String]) -> usize {
        lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.len() - line.trim_start().len())
            .min()
            .unwrap_or(0)
    }

    pub fn lay_out(mut self) -> RegionLayout {
        match self.push_chunks() {
            Ok(()) if self.writes_a_fence_line() => RegionLayout::Prose,
            Ok(()) => RegionLayout::LaidOut(self.laid_out),
            Err(stop) => stop,
        }
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
            if self.rules.syntax.is_arrow_line(&self.lines[index]) {
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
            .map(|line| line.get(self.indent..).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");
        match CodeChunk::new(self.rules.shapes, self.indent, self.layout).lay_out(text.trim_end()) {
            ChunkLayout::LaidOut(lines) => {
                self.laid_out.extend(lines);
                Ok(())
            }
            ChunkLayout::Unreadable => Err(RegionLayout::Unreadable(first)),
            ChunkLayout::Prose => Err(RegionLayout::Prose),
        }
    }

    fn push_result(&mut self, arrow: usize) -> usize {
        self.laid_out.push(self.lines[arrow].clone());
        let mut index = arrow + 1;
        while index < self.lines.len() && self.continues_result(index) {
            self.laid_out.push(self.lines[index].clone());
            index += 1;
        }
        index
    }

    fn continues_result(&self, index: usize) -> bool {
        let line = &self.lines[index];
        let indentation = line.len() - line.trim_start().len();
        !self.is_blank(index) && indentation > self.indent && !self.rules.syntax.is_arrow_line(line)
    }

    fn is_blank(&self, index: usize) -> bool {
        self.lines[index].trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{CodeRegion, RegionLayout};
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
            let lines: Vec<String> = text.split('\n').map(str::to_string).collect();
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
    fn code_that_writes_a_fence_line_is_prose() {
        let fixture = RegionFixture::new();
        assert_eq!(
            fixture.lay_out_spaced("~~~\n(exit 1)\n=> 7", CodeSpacing::Separated),
            RegionLayout::Prose
        );
    }
}

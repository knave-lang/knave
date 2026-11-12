use std::ops::Range;

use super::code_chunk::CodeChunk;
use super::code_layout::CodeLayout;
use super::docstring_regions::DocstringRegions;
use super::shapes::ShapeCatalog;

pub struct CodeRegion<'s, 'l> {
    shapes: &'s ShapeCatalog,
    lines: &'l [String],
    indent: usize,
    layout: CodeLayout,
    laid_out: Vec<String>,
}

impl<'s, 'l> CodeRegion<'s, 'l> {
    pub fn new(shapes: &'s ShapeCatalog, lines: &'l [String], layout: CodeLayout) -> Self {
        Self {
            shapes,
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

    pub fn lay_out(mut self) -> Result<Vec<String>, usize> {
        let mut chunk_start = 0;
        let mut index = 0;
        while index < self.lines.len() {
            if DocstringRegions::is_arrow_line(&self.lines[index]) {
                self.push_chunk(chunk_start..index)?;
                index = self.push_result(index);
                chunk_start = index;
            } else {
                index += 1;
            }
        }
        self.push_chunk(chunk_start..self.lines.len())?;
        Ok(self.laid_out)
    }

    fn push_chunk(&mut self, chunk: Range<usize>) -> Result<(), usize> {
        let Some(first) = chunk.clone().find(|line| !self.is_blank(*line)) else {
            return Ok(());
        };
        let written_blank = first > chunk.start;
        if !self.laid_out.is_empty() && self.layout.spacing.breaks_after_result(written_blank) {
            self.laid_out.push(String::new());
        }
        let text = self.lines[first..chunk.end]
            .iter()
            .map(|line| line.get(self.indent..).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");
        let laid_out = CodeChunk::new(self.shapes, self.indent, self.layout)
            .lay_out(text.trim_end())
            .ok_or(first)?;
        self.laid_out.extend(laid_out);
        Ok(())
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
        !self.is_blank(index) && indentation > self.indent && !DocstringRegions::is_arrow_line(line)
    }

    fn is_blank(&self, index: usize) -> bool {
        self.lines[index].trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::CodeRegion;
    use crate::format::code_layout::CodeLayout;
    use crate::format::code_spacing::CodeSpacing;
    use crate::format::shapes::ShapeCatalog;

    struct RegionFixture {
        shapes: ShapeCatalog,
    }

    impl RegionFixture {
        fn new() -> Self {
            Self {
                shapes: ShapeCatalog::default(),
            }
        }

        fn lay_out(&self, text: &str) -> Result<Vec<String>, usize> {
            self.lay_out_spaced(text, CodeSpacing::Written)
        }

        fn lay_out_spaced(&self, text: &str, spacing: CodeSpacing) -> Result<Vec<String>, usize> {
            let lines: Vec<String> = text.split('\n').map(str::to_string).collect();
            CodeRegion::new(&self.shapes, &lines, CodeLayout { column: 2, spacing }).lay_out()
        }
    }

    #[test]
    fn a_result_line_stays_as_written_under_its_code() {
        let fixture = RegionFixture::new();
        assert_eq!(
            fixture.lay_out("  (f   1)\n\n  =>   (some  1)"),
            Ok(vec!["  (f 1)".to_string(), "  =>   (some  1)".to_string()])
        );
    }

    #[test]
    fn a_deeper_line_after_a_result_continues_it() {
        let fixture = RegionFixture::new();
        assert_eq!(
            fixture.lay_out("(f 1)\n=> (vec 1\n        2)\n(g  2)\n=> 3"),
            Ok(vec![
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
            Ok(vec![
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
        assert_eq!(fixture.lay_out("(f 1)\n=> 1\n\n(g 2\n=> 2"), Err(3));
    }

    #[test]
    fn separated_spacing_puts_a_blank_line_after_a_result() {
        let fixture = RegionFixture::new();
        assert_eq!(
            fixture.lay_out_spaced("(f 1)\n=> 1\n(g 2)\n\n=> 2", CodeSpacing::Separated),
            Ok(vec![
                "(f 1)".to_string(),
                "=> 1".to_string(),
                String::new(),
                "(g 2)".to_string(),
                "=> 2".to_string()
            ])
        );
    }
}

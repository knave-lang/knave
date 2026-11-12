use super::code_layout::CodeLayout;
use super::code_region::CodeRegion;
use super::code_spacing::CodeSpacing;
use super::docstring_line::DocstringLine;
use super::docstring_part::DocstringPart;
use super::docstring_regions::DocstringRegions;
use super::format_rules::FormatRules;
use super::list_plan::Arrangement;
use super::region_layout::RegionLayout;
use super::unreadable_code::UnreadableCode;

pub struct DocstringCode<'s> {
    rules: FormatRules<'s>,
    fence_spacing: CodeSpacing,
}

pub struct LaidOutDocstring {
    pub lines: Vec<String>,
    pub unreadable: Vec<UnreadableCode>,
}

impl<'s> DocstringCode<'s> {
    const LEFT_EDGE: usize = Arrangement::BODY_INDENT;
    const CONTINUATION_START: usize = 1;

    pub fn of_docstrings(rules: FormatRules<'s>) -> Self {
        Self {
            rules,
            fence_spacing: CodeSpacing::Separated,
        }
    }

    pub fn of_pages(rules: FormatRules<'s>) -> Self {
        Self {
            rules,
            fence_spacing: CodeSpacing::Written,
        }
    }

    pub fn lay_out(&self, lines: &[DocstringLine]) -> LaidOutDocstring {
        let continuation = lines.get(Self::CONTINUATION_START..).unwrap_or_default();
        let shared = DocstringLine::least_indentation(continuation);
        let mut laid_out = LaidOutDocstring {
            lines: Vec::new(),
            unreadable: Vec::new(),
        };
        for part in DocstringRegions::parts_of(self.rules.syntax, lines) {
            let range = part.range();
            let code = &lines[range.clone()];
            let Some(spacing) = self.spacing_of(&part) else {
                laid_out.copy(code);
                continue;
            };
            let indent = DocstringLine::least_indentation(code);
            let layout = CodeLayout {
                column: Self::LEFT_EDGE + indent.saturating_sub(shared),
                spacing,
            };
            let region = CodeRegion::new(self.rules, code, layout).lay_out();
            laid_out.add_region(range.start, code, region);
        }
        laid_out
    }

    fn spacing_of(&self, part: &DocstringPart) -> Option<CodeSpacing> {
        match part {
            DocstringPart::Prose(_) => None,
            DocstringPart::Example(_) => Some(CodeSpacing::Written),
            DocstringPart::Fence(_) => Some(self.fence_spacing),
        }
    }
}

impl LaidOutDocstring {
    fn add_region(&mut self, start: usize, code: &[DocstringLine], region: RegionLayout) {
        match region {
            RegionLayout::LaidOut(code_lines) => self.lines.extend(code_lines),
            RegionLayout::Unreadable(unreadable) => {
                self.unreadable.push(UnreadableCode {
                    line: start + unreadable.line,
                    cause: unreadable.cause,
                });
                self.copy(code);
            }
            RegionLayout::Prose => self.copy(code),
        }
    }

    fn copy(&mut self, lines: &[DocstringLine]) {
        self.lines
            .extend(lines.iter().map(|line| line.text().to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::{DocstringCode, DocstringLine};
    use crate::format::docstring_syntax::{DocstringSyntax, DocstringSyntaxFixture};
    use crate::format::format_rules::FormatRules;
    use crate::format::shapes::ShapeCatalog;

    struct DocstringFixture {
        shapes: ShapeCatalog,
        syntax: DocstringSyntax,
    }

    impl DocstringFixture {
        fn new() -> Self {
            Self {
                shapes: ShapeCatalog::default(),
                syntax: DocstringSyntaxFixture::syntax(),
            }
        }

        fn rules(&self) -> FormatRules<'_> {
            FormatRules {
                shapes: &self.shapes,
                syntax: &self.syntax,
            }
        }

        fn lay_out(&self, text: &str) -> (String, Vec<usize>) {
            let lines = DocstringLine::split(text);
            let laid_out = DocstringCode::of_docstrings(self.rules()).lay_out(&lines);
            (
                laid_out.lines.join("\n"),
                laid_out.unreadable.iter().map(|code| code.line).collect(),
            )
        }
    }

    #[test]
    fn a_page_fence_keeps_its_written_spacing() {
        let fixture = DocstringFixture::new();
        let lines = DocstringLine::split("# page\n\n```knave\n(f   1)\n(g 2)\n```");
        let laid_out = DocstringCode::of_pages(fixture.rules()).lay_out(&lines);
        assert_eq!(
            laid_out.lines,
            ["# page", "", "```knave", "(f 1)", "(g 2)", "```"]
        );
    }

    #[test]
    fn prose_stays_and_code_lays_out() {
        let fixture = DocstringFixture::new();
        assert_eq!(
            fixture.lay_out("Pads   it.\n\n  Some   prose.\n\n  (pad   1)\n  => 1"),
            (
                "Pads   it.\n\n  Some   prose.\n\n  (pad 1)\n  => 1".to_string(),
                vec![]
            )
        );
    }

    #[test]
    fn code_that_does_not_read_stays_and_names_its_line() {
        let fixture = DocstringFixture::new();
        assert_eq!(
            fixture.lay_out("Pads.\n\n  ```\n  (pad   1\n  ```"),
            ("Pads.\n\n  ```\n  (pad   1\n  ```".to_string(), vec![3])
        );
    }

    #[test]
    fn code_lays_out_as_if_the_docstring_sat_at_the_body_indent() {
        let fixture = DocstringFixture::new();
        let atom = "a".repeat(72);
        let (laid_out, _) = fixture.lay_out(&format!(
            "Calls.\n\n      ```\n      (call {atom})\n      ```"
        ));
        assert_eq!(
            laid_out.split('\n').skip(3).take(2).collect::<Vec<_>>(),
            ["      (call", &format!("        {atom})")]
        );
    }
}

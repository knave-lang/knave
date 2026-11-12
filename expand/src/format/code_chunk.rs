use super::code_layout::CodeLayout;
use super::layout::LayoutEngine;
use super::region_layout::RegionLayout;
use super::shapes::ShapeCatalog;
use super::syntax_tree::SyntaxTree;
use super::top_level_printer::TopLevelPrinter;
use crate::reader::{Form, Reader};
use crate::source::SourceMap;

pub struct CodeChunk<'s> {
    shapes: &'s ShapeCatalog,
    indent: usize,
    layout: CodeLayout,
}

impl<'s> CodeChunk<'s> {
    const SOURCE_NAME: &'static str = "<docstring code>";
    const FIRST_LINE: usize = 0;

    pub fn new(shapes: &'s ShapeCatalog, indent: usize, layout: CodeLayout) -> Self {
        Self {
            shapes,
            indent,
            layout,
        }
    }

    pub fn lay_out(self, text: &str) -> RegionLayout {
        let Some(forms) = Self::read(text) else {
            return RegionLayout::Unreadable(Self::FIRST_LINE);
        };
        if self.layout.spacing.needs_a_line_per_form() && !Self::ends_each_line(text, &forms) {
            return RegionLayout::Prose;
        }
        let mut engine = LayoutEngine::new(self.shapes);
        let lines = TopLevelPrinter::new(&mut engine, self.layout).print(&SyntaxTree::build(text));
        RegionLayout::LaidOut(lines.iter().map(|line| self.with_margin(line)).collect())
    }

    fn read(text: &str) -> Option<Vec<Form>> {
        let mut sources = SourceMap::default();
        let source = sources.add(Self::SOURCE_NAME, text);
        Reader::new(sources.file(source)).read().ok()
    }

    fn ends_each_line(text: &str, forms: &[Form]) -> bool {
        forms.windows(2).all(|pair| {
            let between = pair[0].span().range().end..pair[1].span().range().start;
            text[between].contains('\n')
        })
    }

    fn with_margin(&self, line: &str) -> String {
        if line.is_empty() {
            return String::new();
        }
        let text = line.get(self.layout.column..).unwrap_or(line.trim_start());
        format!("{}{text}", " ".repeat(self.indent))
    }
}

#[cfg(test)]
mod tests {
    use super::{CodeChunk, RegionLayout};
    use crate::format::code_layout::CodeLayout;
    use crate::format::code_spacing::CodeSpacing;
    use crate::format::shapes::ShapeCatalog;

    struct ChunkFixture {
        shapes: ShapeCatalog,
    }

    impl ChunkFixture {
        fn new() -> Self {
            Self {
                shapes: ShapeCatalog::default(),
            }
        }

        fn lay_out(&self, text: &str) -> Option<Vec<String>> {
            self.lay_out_spaced(text, CodeSpacing::Written)
        }

        fn lay_out_spaced(&self, text: &str, spacing: CodeSpacing) -> Option<Vec<String>> {
            match self.layout(text, spacing) {
                RegionLayout::LaidOut(lines) => Some(lines),
                RegionLayout::Unreadable(_) | RegionLayout::Prose => None,
            }
        }

        fn layout(&self, text: &str, spacing: CodeSpacing) -> RegionLayout {
            let layout = CodeLayout { column: 2, spacing };
            CodeChunk::new(&self.shapes, 4, layout).lay_out(text)
        }
    }

    #[test]
    fn each_form_takes_its_own_lines_at_the_chunk_indent() {
        let fixture = ChunkFixture::new();
        assert_eq!(
            fixture.lay_out("(f   1)\n(g 2) ; why"),
            Some(vec!["    (f 1)".to_string(), "    (g 2) ; why".to_string()])
        );
    }

    #[test]
    fn a_written_blank_line_between_forms_stays() {
        let fixture = ChunkFixture::new();
        assert_eq!(
            fixture.lay_out("(f 1)\n\n\n(g 2)"),
            Some(vec![
                "    (f 1)".to_string(),
                String::new(),
                "    (g 2)".to_string()
            ])
        );
    }

    #[test]
    fn comments_keep_their_lines() {
        let fixture = ChunkFixture::new();
        assert_eq!(
            fixture.lay_out("; first\n\n(f 1)\n\n; last"),
            Some(vec![
                "    ; first".to_string(),
                String::new(),
                "    (f 1)".to_string(),
                String::new(),
                "    ; last".to_string()
            ])
        );
    }

    #[test]
    fn a_long_form_breaks_as_it_would_at_the_chunk_column() {
        let fixture = ChunkFixture::new();
        let text = format!("(call {} {})", "a".repeat(40), "b".repeat(40));
        assert_eq!(
            fixture.lay_out(&text),
            Some(vec![
                format!("    (call {}", "a".repeat(40)),
                format!("          {})", "b".repeat(40))
            ])
        );
    }

    #[test]
    fn code_that_does_not_read_is_unreadable() {
        let fixture = ChunkFixture::new();
        assert!(matches!(
            fixture.layout("(f 1", CodeSpacing::Written),
            RegionLayout::Unreadable(0)
        ));
    }

    #[test]
    fn an_example_with_two_forms_on_a_line_is_prose() {
        let fixture = ChunkFixture::new();
        assert!(matches!(
            fixture.layout("(see below) and more", CodeSpacing::Written),
            RegionLayout::Prose
        ));
    }

    #[test]
    fn a_fence_with_two_forms_on_a_line_lays_them_out() {
        let fixture = ChunkFixture::new();
        assert_eq!(
            fixture.lay_out_spaced("(f 1) (g 2)", CodeSpacing::Separated),
            Some(vec![
                "    (f 1)".to_string(),
                String::new(),
                "    (g 2)".to_string()
            ])
        );
    }

    #[test]
    fn separated_spacing_puts_a_blank_line_between_forms() {
        let fixture = ChunkFixture::new();
        assert_eq!(
            fixture.lay_out_spaced("(f 1)\n(g 2)", CodeSpacing::Separated),
            Some(vec![
                "    (f 1)".to_string(),
                String::new(),
                "    (g 2)".to_string()
            ])
        );
    }

    #[test]
    fn separated_spacing_keeps_one_line_imports_together() {
        let fixture = ChunkFixture::new();
        assert_eq!(
            fixture.lay_out_spaced("(use m a)\n(use n b)\n(f 1)", CodeSpacing::Separated),
            Some(vec![
                "    (use m a)".to_string(),
                "    (use n b)".to_string(),
                String::new(),
                "    (f 1)".to_string()
            ])
        );
    }

    #[test]
    fn separated_spacing_puts_a_blank_line_before_a_comment_that_leads_a_form() {
        let fixture = ChunkFixture::new();
        assert_eq!(
            fixture.lay_out_spaced("(use m a)\n; why\n(use n b)", CodeSpacing::Separated),
            Some(vec![
                "    (use m a)".to_string(),
                String::new(),
                "    ; why".to_string(),
                "    (use n b)".to_string()
            ])
        );
    }

    #[test]
    fn a_blank_line_between_two_leading_comments_stays() {
        let fixture = ChunkFixture::new();
        assert_eq!(
            fixture.lay_out("(f 1)\n; first\n\n; second\n(g 2)"),
            Some(vec![
                "    (f 1)".to_string(),
                "    ; first".to_string(),
                String::new(),
                "    ; second".to_string(),
                "    (g 2)".to_string()
            ])
        );
    }

    #[test]
    fn a_blank_line_before_a_leading_comment_stays() {
        let fixture = ChunkFixture::new();
        assert_eq!(
            fixture.lay_out("(f 1)\n\n; why\n(g 2)"),
            Some(vec![
                "    (f 1)".to_string(),
                String::new(),
                "    ; why".to_string(),
                "    (g 2)".to_string()
            ])
        );
    }
}

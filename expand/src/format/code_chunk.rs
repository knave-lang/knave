use super::block::Block;
use super::code_layout::CodeLayout;
use super::import_group::ImportGroup;
use super::layout::{LaidOut, LayoutEngine};
use super::shapes::ShapeCatalog;
use super::syntax_tree::{Comment, Entry, SyntaxTree};
use crate::reader::Reader;
use crate::source::SourceMap;

pub struct CodeChunk<'s> {
    shapes: &'s ShapeCatalog,
    indent: usize,
    layout: CodeLayout,
    lines: Vec<String>,
    previous_group: Option<ImportGroup>,
}

impl<'s> CodeChunk<'s> {
    const SOURCE_NAME: &'static str = "<docstring code>";

    pub fn new(shapes: &'s ShapeCatalog, indent: usize, layout: CodeLayout) -> Self {
        Self {
            shapes,
            indent,
            layout,
            lines: Vec::new(),
            previous_group: None,
        }
    }

    pub fn lay_out(mut self, text: &str) -> Option<Vec<String>> {
        Self::reads(text).then_some(())?;
        let tree = SyntaxTree::build(text);
        for (position, entry) in tree.entries.iter().enumerate() {
            self.push_entry(entry, position);
        }
        self.push_closing_comments(&tree.closing_comments);
        Some(self.lines)
    }

    fn reads(text: &str) -> bool {
        let mut sources = SourceMap::default();
        let source = sources.add(Self::SOURCE_NAME, text);
        Reader::new(sources.file(source)).read().is_ok()
    }

    fn push_entry(&mut self, entry: &Entry, position: usize) {
        let block = self.block_of(entry);
        let group = ImportGroup::of(entry, &block);
        let comments_first = !entry.leading_comments.is_empty();
        let joins_group = group.is_some() && group == self.previous_group && !comments_first;
        let written_blank = entry
            .leading_comments
            .first()
            .map_or(entry.blank_before_node, |comment| comment.blank_before);
        if position > 0
            && self
                .layout
                .spacing
                .breaks_before_form(written_blank, joins_group)
        {
            self.lines.push(String::new());
        }
        self.push_leading_comments(&entry.leading_comments);
        if comments_first && entry.blank_before_node {
            self.lines.push(String::new());
        }
        let mut rendered = String::new();
        block.render_into(&mut rendered);
        self.push_rendered(&rendered);
        self.previous_group = group;
    }

    fn block_of(&self, entry: &Entry) -> Block {
        let mut block: Block =
            LayoutEngine::new(self.shapes).node(&entry.node, self.layout.column, 0);
        if let Some(comment) = &entry.trailing_comment {
            block.add_trailing_comment(comment);
        }
        block
    }

    fn push_leading_comments(&mut self, comments: &[Comment]) {
        for (position, comment) in comments.iter().enumerate() {
            if position > 0 && comment.blank_before {
                self.lines.push(String::new());
            }
            self.push_rendered(&comment.text);
        }
    }

    fn push_closing_comments(&mut self, comments: &[Comment]) {
        for (position, comment) in comments.iter().enumerate() {
            let breaks = match position {
                0 => self
                    .layout
                    .spacing
                    .breaks_before_form(comment.blank_before, false),
                _ => comment.blank_before,
            };
            if breaks && !self.lines.is_empty() {
                self.lines.push(String::new());
            }
            self.push_rendered(&comment.text);
        }
    }

    fn push_rendered(&mut self, rendered: &str) {
        let margin = " ".repeat(self.indent);
        for (index, line) in rendered.split('\n').enumerate() {
            let text = if index == 0 {
                line
            } else {
                line.get(self.layout.column..).unwrap_or(line.trim_start())
            };
            self.lines.push(format!("{margin}{text}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CodeChunk;
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
            let layout = CodeLayout { column: 2, spacing };
            CodeChunk::new(&self.shapes, 4, layout).lay_out(text)
        }
    }

    #[test]
    fn each_form_takes_its_own_lines_at_the_chunk_indent() {
        let fixture = ChunkFixture::new();
        assert_eq!(
            fixture.lay_out("(f   1)  (g 2) ; why"),
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
    fn code_that_does_not_read_lays_out_to_nothing() {
        assert_eq!(ChunkFixture::new().lay_out("(f 1"), None);
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
}

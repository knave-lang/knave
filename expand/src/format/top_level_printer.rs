use super::block::Block;
use super::code_layout::CodeLayout;
use super::import_group::ImportGroup;
use super::import_join::ImportJoin;
use super::layout::{LaidOut, LayoutEngine};
use super::syntax_tree::{Comment, Entry, SyntaxTree};

pub struct TopLevelPrinter<'e, 's> {
    engine: &'e mut LayoutEngine<'s>,
    layout: CodeLayout,
    lines: Vec<String>,
    previous_group: Option<ImportGroup>,
}

impl<'e, 's> TopLevelPrinter<'e, 's> {
    pub fn new(engine: &'e mut LayoutEngine<'s>, layout: CodeLayout) -> Self {
        Self {
            engine,
            layout,
            lines: Vec::new(),
            previous_group: None,
        }
    }

    pub fn print(mut self, tree: &SyntaxTree) -> Vec<String> {
        for (position, entry) in tree.entries.iter().enumerate() {
            self.print_entry(entry, position);
        }
        self.print_closing_comments(&tree.closing_comments);
        self.lines
    }

    fn print_entry(&mut self, entry: &Entry, position: usize) {
        let block = self.block_of(entry);
        let group = ImportGroup::of(entry, &block);
        let join = self.join_of(group, entry);
        if position > 0
            && self
                .layout
                .spacing
                .breaks_before_form(entry.gap_before(), join)
        {
            self.lines.push(String::new());
        }
        self.print_comments(&entry.leading_comments);
        if !entry.leading_comments.is_empty() && entry.blank_before_node {
            self.lines.push(String::new());
        }
        let mut rendered = String::new();
        block.render_into(&mut rendered);
        self.push_rendered(&rendered);
        self.previous_group = group;
    }

    fn block_of(&mut self, entry: &Entry) -> Block {
        let mut block: Block = self.engine.node(&entry.node, self.layout.column, 0);
        if let Some(comment) = &entry.trailing_comment {
            block.add_trailing_comment(comment);
        }
        block
    }

    fn join_of(&self, group: Option<ImportGroup>, entry: &Entry) -> ImportJoin {
        if group.is_some() && group == self.previous_group && entry.leading_comments.is_empty() {
            ImportJoin::JoinsGroup
        } else {
            ImportJoin::Apart
        }
    }

    fn print_comments(&mut self, comments: &[Comment]) {
        for (position, comment) in comments.iter().enumerate() {
            if position > 0 && comment.blank_before {
                self.lines.push(String::new());
            }
            self.push_rendered(&comment.text);
        }
    }

    fn print_closing_comments(&mut self, comments: &[Comment]) {
        for (position, comment) in comments.iter().enumerate() {
            let breaks = match position {
                0 => self
                    .layout
                    .spacing
                    .breaks_before_form(comment.gap_before(), ImportJoin::Apart),
                _ => comment.blank_before,
            };
            if breaks && !self.lines.is_empty() {
                self.lines.push(String::new());
            }
            self.push_rendered(&comment.text);
        }
    }

    fn push_rendered(&mut self, rendered: &str) {
        let indent = " ".repeat(self.layout.column);
        for (index, line) in rendered.split('\n').enumerate() {
            let text = if index == 0 {
                format!("{indent}{line}")
            } else {
                line.to_string()
            };
            self.lines.push(text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TopLevelPrinter;
    use crate::format::code_layout::CodeLayout;
    use crate::format::code_spacing::CodeSpacing;
    use crate::format::layout::LayoutEngine;
    use crate::format::shapes::ShapeCatalog;
    use crate::format::syntax_tree::SyntaxTree;

    struct PrinterFixture {
        shapes: ShapeCatalog,
    }

    impl PrinterFixture {
        fn new() -> Self {
            Self {
                shapes: ShapeCatalog::default(),
            }
        }

        fn print(&self, text: &str) -> Vec<String> {
            let mut engine = LayoutEngine::new(&self.shapes);
            let layout = CodeLayout {
                column: 0,
                spacing: CodeSpacing::Separated,
            };
            TopLevelPrinter::new(&mut engine, layout).print(&SyntaxTree::build(text))
        }
    }

    #[test]
    fn closing_comments_keep_the_blank_lines_written_between_them() {
        let fixture = PrinterFixture::new();
        assert_eq!(
            fixture.print("(f 1)\n; first\n; second\n\n; third"),
            ["(f 1)", "", "; first", "; second", "", "; third"]
        );
    }
}

use super::docstring_layouts::DocstringLayouts;
use super::format_rules::FormatRules;
use super::syntax_tree::{Atom, Entry, ListNode, SyntaxNode, SyntaxTree};
use super::unreadable_span::UnreadableSpan;
use crate::stack::DeepRecursion;

pub struct TreeDocstrings<'s, 'm> {
    rules: FormatRules<'s>,
    layouts: &'m mut DocstringLayouts<'s>,
    unreadable: Vec<UnreadableSpan>,
}

impl<'s, 'm> TreeDocstrings<'s, 'm> {
    pub fn new(layouts: &'m mut DocstringLayouts<'s>) -> Self {
        Self {
            rules: layouts.rules(),
            layouts,
            unreadable: Vec::new(),
        }
    }

    pub fn lay_out(mut self, tree: &mut SyntaxTree) -> Vec<UnreadableSpan> {
        self.visit_entries(&mut tree.entries);
        self.unreadable
    }

    fn visit_entries(&mut self, entries: &mut [Entry]) {
        for entry in entries {
            if let SyntaxNode::List(list) = &mut entry.node {
                DeepRecursion::guard(|| self.visit_list(list));
            }
        }
    }

    fn visit_list(&mut self, list: &mut ListNode) {
        let docstring = self.rules.shapes.docstring_entry(list);
        if let Some(SyntaxNode::Atom(atom)) = docstring.map(|entry| &mut list.entries[entry].node) {
            self.lay_out_docstring(atom);
        }
        self.visit_entries(&mut list.entries);
    }

    fn lay_out_docstring(&mut self, atom: &mut Atom) {
        let laid_out = self.layouts.of_token(&atom.text);
        for span in &laid_out.unreadable {
            self.unreadable.push(span.after(atom.start));
        }
        atom.text = laid_out.text.clone();
    }
}

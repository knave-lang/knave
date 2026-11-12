use std::ops::Range;

use super::docstring_code::DocstringCode;
use super::docstring_lines::DocstringLines;
use super::format_rules::FormatRules;
use super::syntax_tree::{Atom, Entry, ListNode, SyntaxNode, SyntaxTree};
use super::unreadable_cause::UnreadableCause;
use crate::stack::DeepRecursion;

pub struct TreeDocstrings<'s> {
    rules: FormatRules<'s>,
    unreadable: Vec<(Range<usize>, UnreadableCause)>,
}

impl<'s> TreeDocstrings<'s> {
    pub fn new(rules: FormatRules<'s>) -> Self {
        Self {
            rules,
            unreadable: Vec::new(),
        }
    }

    pub fn lay_out(mut self, tree: &mut SyntaxTree) -> Vec<(Range<usize>, UnreadableCause)> {
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
        let Some(lines) = DocstringLines::of_token(&atom.text) else {
            return;
        };
        let laid_out = DocstringCode::of_docstrings(self.rules).lay_out(lines.lines());
        for code in laid_out.unreadable {
            let range = lines.written_range(code.line);
            self.unreadable
                .push((atom.start + range.start..atom.start + range.end, code.cause));
        }
        atom.text = DocstringLines::token_of(&laid_out.lines);
    }
}

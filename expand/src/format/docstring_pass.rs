use super::docstring_code::DocstringCode;
use super::docstring_lines::DocstringLines;
use super::format_problems::FormatProblems;
use super::format_rules::FormatRules;
use super::syntax_tree::{Atom, Entry, ListNode, SyntaxNode, SyntaxTree};
use crate::diagnostic::Diagnostics;
use crate::source::{SourceId, Span};
use crate::stack::DeepRecursion;

pub struct DocstringPass<'s> {
    rules: FormatRules<'s>,
    source: SourceId,
    warnings: Diagnostics,
}

impl<'s> DocstringPass<'s> {
    pub fn new(rules: FormatRules<'s>, source: SourceId) -> Self {
        Self {
            rules,
            source,
            warnings: Diagnostics::default(),
        }
    }

    pub fn lay_out_docstrings(mut self, tree: &mut SyntaxTree) -> Diagnostics {
        self.visit_entries(&mut tree.entries);
        self.warnings
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
        for line in laid_out.unreadable_lines {
            let range = lines.written_range(line);
            let span = Span::new(
                self.source,
                atom.start + range.start..atom.start + range.end,
            );
            self.warnings
                .push(FormatProblems::unreadable_docstring_code(span));
        }
        atom.text = DocstringLines::token_of(&laid_out.lines);
    }
}

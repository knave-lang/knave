use std::ops::Range;

use super::docstring_code::DocstringCode;
use super::docstring_lines::DocstringLines;
use super::docstring_memo::{DocstringMemo, LaidOutToken};
use super::format_rules::FormatRules;
use super::syntax_tree::{Atom, Entry, ListNode, SyntaxNode, SyntaxTree};
use super::unreadable_cause::UnreadableCause;
use crate::stack::DeepRecursion;

pub struct TreeDocstrings<'s, 'm> {
    rules: FormatRules<'s>,
    memo: &'m mut DocstringMemo,
    unreadable: Vec<(Range<usize>, UnreadableCause)>,
}

impl<'s, 'm> TreeDocstrings<'s, 'm> {
    pub fn new(rules: FormatRules<'s>, memo: &'m mut DocstringMemo) -> Self {
        Self {
            rules,
            memo,
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
        let rules = self.rules;
        let laid_out = self
            .memo
            .laid_out(&atom.text, || Self::lay_out_token(rules, &atom.text));
        for (range, cause) in &laid_out.unreadable {
            self.unreadable
                .push((atom.start + range.start..atom.start + range.end, *cause));
        }
        atom.text = laid_out.text.clone();
    }

    fn lay_out_token(rules: FormatRules<'_>, token: &str) -> LaidOutToken {
        let Some(lines) = DocstringLines::of_token(token) else {
            return LaidOutToken {
                text: token.to_string(),
                unreadable: Vec::new(),
            };
        };
        let laid_out = DocstringCode::of_docstrings(rules).lay_out(lines.lines());
        LaidOutToken {
            text: DocstringLines::token_of(&laid_out.lines),
            unreadable: laid_out
                .unreadable
                .iter()
                .map(|code| (lines.written_range(code.line), code.cause))
                .collect(),
        }
    }
}

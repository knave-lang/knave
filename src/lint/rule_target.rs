use std::rc::Rc;

use super::rule_call::RuleCall;
use super::rule_entries::RuleEntries;
use crate::interpret::Value;
use crate::reader::Form;
use crate::source::Span;

pub struct RuleTarget {
    entry: TargetEntry,
    table: Value,
    name: Rc<str>,
    span: Span,
}

enum TargetEntry {
    Definition,
    Module,
}

impl RuleTarget {
    pub fn for_definition(table: Value, name: Rc<str>, span: Span) -> Self {
        Self {
            entry: TargetEntry::Definition,
            table,
            name,
            span,
        }
    }

    pub fn for_module(table: Value, name: Rc<str>, span: Span) -> Self {
        Self {
            entry: TargetEntry::Module,
            table,
            name,
            span,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn into_table(self) -> Value {
        self.table
    }

    pub fn call(&self, rule: Form, entries: RuleEntries, step_limit: u64) -> RuleCall {
        let entry = match self.entry {
            TargetEntry::Definition => entries.check,
            TargetEntry::Module => entries.check_module,
        };
        RuleCall {
            entry,
            rule,
            table: self.table.clone(),
            span: self.span,
            step_limit,
        }
    }
}

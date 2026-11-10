use std::ops::Range;

use super::Forms;
use crate::diagnostic::Diagnostic;
use crate::interpret::arguments::Arguments;
use crate::interpret::problems::InterpretProblems;
use crate::interpret::{BuildLimit, CodeList, NestedTooDeep, Value};
use crate::source::Span;

pub struct ListOperations<'a> {
    arguments: &'a Arguments<'a>,
    call_site: Span,
}

impl<'a> ListOperations<'a> {
    pub fn new(arguments: &'a Arguments<'a>, call_site: Span) -> Self {
        Self {
            arguments,
            call_site,
        }
    }

    pub fn first(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        self.item(&self.arguments.list(0)?, 0)
    }

    pub fn rest(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        self.dropped(&self.arguments.list(0)?, 1)
    }

    pub fn list(&self) -> Result<Value, Diagnostic> {
        let values = (0..self.arguments.values().len())
            .map(|index| self.arguments.code(index))
            .collect::<Result<Vec<Value>, Diagnostic>>()?;
        self.arguments.charge(values.len())?;
        self.built(CodeList::new(values, self.call_site))
    }

    pub fn prepend(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(2)?;
        let item = self.arguments.code(0)?;
        let items = self.arguments.list(1)?;
        self.arguments.charge(items.len() + 1)?;
        self.built(CodeList::new(
            [vec![item], items.values()].concat(),
            self.call_site,
        ))
    }

    pub fn concat(&self) -> Result<Value, Diagnostic> {
        let lists = (0..self.arguments.values().len())
            .map(|index| self.arguments.list(index))
            .collect::<Result<Vec<CodeList>, Diagnostic>>()?;
        let Some((head, tail)) = lists.split_first() else {
            return self.built(CodeList::new(Vec::new(), self.call_site));
        };
        let added = tail.iter().map(CodeList::len).sum();
        self.arguments.charge(head.append_work(added))?;
        let extra = tail.iter().flat_map(CodeList::values).collect();
        self.built(head.appended(extra, self.call_site))
    }

    pub fn len(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        Ok(Forms::count(self.arguments.list(0)?.len(), self.call_site))
    }

    pub fn is_empty(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        Ok(Value::Bool(self.arguments.list(0)?.is_empty()))
    }

    pub fn nth(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(2)?;
        let items = self.arguments.list(0)?;
        self.item(&items, self.arguments.index(1)?)
    }

    pub fn take(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(2)?;
        let items = self.arguments.list(0)?;
        let count = self.arguments.index(1)?;
        self.sliced(&items, 0..count)
    }

    pub fn drop(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(2)?;
        let count = self.arguments.index(1)?;
        self.dropped(&self.arguments.list(0)?, count)
    }

    fn built(&self, list: Result<CodeList, NestedTooDeep>) -> Result<Value, Diagnostic> {
        let limit = BuildLimit::of(self.arguments);
        let list = list.map_err(|deep| deep.at(limit.span()))?;
        Ok(Value::List(limit.check(list)?))
    }

    fn item(&self, items: &CodeList, index: usize) -> Result<Value, Diagnostic> {
        items.get(index).ok_or_else(|| self.out_of_range())
    }

    fn dropped(&self, items: &CodeList, count: usize) -> Result<Value, Diagnostic> {
        self.sliced(items, count..items.len())
    }

    fn sliced(&self, items: &CodeList, range: Range<usize>) -> Result<Value, Diagnostic> {
        items
            .slice(range, self.call_site)
            .map(Value::List)
            .ok_or_else(|| self.out_of_range())
    }

    fn out_of_range(&self) -> Diagnostic {
        InterpretProblems::out_of_range(self.arguments.function(), self.arguments.span())
    }
}

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use super::Value;
use super::limit_problems::LimitProblems;
use crate::diagnostic::Diagnostic;
use crate::reader::{Form, FormKind, FormNesting};
use crate::source::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassedLimit {
    FormCount,
    Nesting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NestedTooDeep;

impl NestedTooDeep {
    pub fn at(self, span: Span) -> Diagnostic {
        LimitProblems::value_nests_too_deep(ListNesting::LIMIT, span)
    }
}

pub struct ListNesting;

impl ListNesting {
    pub const LIMIT: usize = 4 * Form::NESTING_LIMIT;

    pub fn form_list(items: Vec<Form>, span: Span) -> Result<Value, NestedTooDeep> {
        let list = Form::new(FormKind::List(items), span);
        Self::admitted(FormNesting::of(&list))?;
        Ok(Value::Form(list))
    }

    fn of_items(values: &[Value]) -> Result<u32, NestedTooDeep> {
        Self::admitted(1 + values.iter().map(Self::of_value).max().unwrap_or(0))
    }

    fn of_value(value: &Value) -> u32 {
        match value {
            Value::Form(form) => FormNesting::of(form),
            Value::List(list) => list.nesting,
            _ => 0,
        }
    }

    fn admitted(nesting: u32) -> Result<u32, NestedTooDeep> {
        if nesting as usize > Self::LIMIT {
            Err(NestedTooDeep)
        } else {
            Ok(nesting)
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CodeList {
    items: ListItems,
    range: Range<usize>,
    span: Span,
    nesting: u32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
enum ListItems {
    Read(Form),
    Built(Rc<RefCell<Vec<Value>>>),
}

impl CodeList {
    pub fn new(values: Vec<Value>, span: Span) -> Result<Self, NestedTooDeep> {
        let nesting = ListNesting::of_items(&values)?;
        Ok(Self {
            range: 0..values.len(),
            items: ListItems::Built(Rc::new(RefCell::new(values))),
            span,
            nesting,
        })
    }

    pub fn of_form(form: &Form) -> Option<Self> {
        let length = form.as_list()?.len();
        Some(Self {
            items: ListItems::Read(form.clone()),
            range: 0..length,
            span: form.span(),
            nesting: FormNesting::of(form),
        })
    }

    pub fn len(&self) -> usize {
        self.range.len()
    }

    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<Value> {
        (index < self.len()).then(|| self.item(self.range.start + index))
    }

    pub fn slice(&self, range: Range<usize>, span: Span) -> Option<Self> {
        (range.start <= range.end && range.end <= self.len()).then(|| Self {
            items: self.items.clone(),
            range: self.range.start + range.start..self.range.start + range.end,
            span,
            nesting: self.nesting,
        })
    }

    pub fn appended(&self, extra: Vec<Value>, span: Span) -> Result<Self, NestedTooDeep> {
        match self.growable_buffer() {
            Some(buffer) => Self::grown(self, buffer, extra, span),
            None => Self::new([self.values(), extra].concat(), span),
        }
    }

    pub fn prepended(&self, value: Value, span: Span) -> Result<Self, NestedTooDeep> {
        Self::new([vec![value], self.values()].concat(), span)
    }

    pub fn values(&self) -> Vec<Value> {
        self.range.clone().map(|index| self.item(index)).collect()
    }

    pub fn forms(&self) -> Vec<Form> {
        self.values()
            .iter()
            .map(|value| value.to_form().expect("a code list holds only code"))
            .collect()
    }

    pub fn to_form(&self) -> Form {
        Form::new(FormKind::List(self.forms()), self.span)
    }

    pub fn to_form_within(
        &self,
        allowance: &mut usize,
        enclosing_lists: usize,
    ) -> Result<Form, PassedLimit> {
        if enclosing_lists == Form::NESTING_LIMIT {
            return Err(PassedLimit::Nesting);
        }
        *allowance = allowance.checked_sub(1).ok_or(PassedLimit::FormCount)?;
        let mut forms = Vec::with_capacity(self.len());
        for index in self.range.clone() {
            forms.push(
                self.item(index)
                    .to_form_within(allowance, enclosing_lists + 1)?,
            );
        }
        Ok(Form::new(FormKind::List(forms), self.span))
    }

    fn grown(
        list: &Self,
        buffer: &Rc<RefCell<Vec<Value>>>,
        extra: Vec<Value>,
        span: Span,
    ) -> Result<Self, NestedTooDeep> {
        let nesting = list.nesting.max(ListNesting::of_items(&extra)?);
        buffer.borrow_mut().extend(extra);
        Ok(Self {
            range: list.range.start..buffer.borrow().len(),
            items: ListItems::Built(Rc::clone(buffer)),
            span,
            nesting,
        })
    }

    fn item(&self, index: usize) -> Value {
        match &self.items {
            ListItems::Read(form) => {
                Value::Form(form.as_list().expect("read as a list")[index].clone())
            }
            ListItems::Built(buffer) => buffer.borrow()[index].clone(),
        }
    }

    fn growable_buffer(&self) -> Option<&Rc<RefCell<Vec<Value>>>> {
        match &self.items {
            ListItems::Built(buffer) if buffer.borrow().len() == self.range.end => Some(buffer),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CodeList, ListNesting, NestedTooDeep, PassedLimit};
    use crate::interpret::Value;
    use crate::reader::{Form, FormKind, Symbol};
    use crate::source::{SourceId, Span};

    struct NestedList;

    impl NestedList {
        const NO_ENCLOSING_LISTS: usize = 0;

        fn span() -> Span {
            Span::new(SourceId::new(0), 0..1)
        }

        fn leaf() -> Value {
            Value::Form(Form::new(
                FormKind::Symbol(Symbol::new("leaf")),
                Self::span(),
            ))
        }

        fn list_of(items: Vec<Value>) -> CodeList {
            CodeList::new(items, Self::span()).expect("within the limit")
        }

        fn of_depth(depth: usize) -> Value {
            Self::built_to(depth).expect("within the limit")
        }

        fn built_to(depth: usize) -> Result<Value, NestedTooDeep> {
            let innermost = Value::List(CodeList::new(Vec::new(), Self::span())?);
            (1..depth).try_fold(innermost, |inner, _| {
                CodeList::new(vec![inner], Self::span()).map(Value::List)
            })
        }

        fn converted(value: &Value) -> Result<Form, PassedLimit> {
            let mut allowance = usize::MAX;
            value.to_form_within(&mut allowance, Self::NO_ENCLOSING_LISTS)
        }
    }

    #[test]
    fn a_built_list_at_the_nesting_limit_becomes_a_form() {
        let value = NestedList::of_depth(Form::NESTING_LIMIT);
        assert!(NestedList::converted(&value).is_ok());
    }

    #[test]
    fn a_built_list_one_past_the_nesting_limit_nests_too_deeply() {
        let value = NestedList::of_depth(Form::NESTING_LIMIT + 1);
        let refusal = NestedList::converted(&value).err();
        assert_eq!(refusal, Some(PassedLimit::Nesting));
    }

    #[test]
    fn a_built_list_at_the_built_limit_is_admitted() {
        assert!(NestedList::built_to(ListNesting::LIMIT).is_ok());
    }

    #[test]
    fn a_built_list_one_past_the_built_limit_is_refused() {
        assert_eq!(
            NestedList::built_to(ListNesting::LIMIT + 1).err(),
            Some(NestedTooDeep)
        );
    }

    #[test]
    fn a_form_list_past_the_built_limit_is_refused() {
        let inner = Form::new(FormKind::List(Vec::new()), NestedList::span());
        let deep = (1..ListNesting::LIMIT).fold(inner, |form, _| {
            Form::new(FormKind::List(vec![form]), NestedList::span())
        });
        assert_eq!(
            ListNesting::form_list(vec![deep], NestedList::span()).err(),
            Some(NestedTooDeep)
        );
    }

    #[test]
    fn appending_counts_the_deepest_new_item() {
        let shallow = NestedList::list_of(vec![NestedList::leaf()]);
        let deep = NestedList::of_depth(ListNesting::LIMIT);
        assert_eq!(
            shallow.appended(vec![deep], NestedList::span()).err(),
            Some(NestedTooDeep)
        );
    }

    #[test]
    fn appending_to_a_shared_prefix_counts_the_deepest_new_item() {
        let list = NestedList::list_of(vec![NestedList::leaf(), NestedList::leaf()]);
        let prefix = list.slice(0..1, NestedList::span()).expect("in range");
        let deep = NestedList::of_depth(ListNesting::LIMIT);
        assert_eq!(
            prefix.appended(vec![deep], NestedList::span()).err(),
            Some(NestedTooDeep)
        );
    }

    #[test]
    fn prepending_counts_the_new_item() {
        let list = NestedList::list_of(vec![NestedList::leaf()]);
        let deep = NestedList::of_depth(ListNesting::LIMIT);
        assert_eq!(
            list.prepended(deep, NestedList::span()).err(),
            Some(NestedTooDeep)
        );
    }

    #[test]
    fn a_slice_keeps_the_nesting_of_its_list() {
        let deep = NestedList::of_depth(ListNesting::LIMIT - 1);
        let list = NestedList::list_of(vec![NestedList::leaf(), deep]);
        let slice = list.slice(0..1, NestedList::span()).expect("in range");
        let wrapped = CodeList::new(vec![Value::List(slice)], NestedList::span());
        assert_eq!(wrapped.err(), Some(NestedTooDeep));
    }

    #[test]
    fn a_read_list_takes_its_nesting_from_its_form() {
        let inner = Form::new(FormKind::List(Vec::new()), NestedList::span());
        let deep = (1..ListNesting::LIMIT).fold(inner, |form, _| {
            Form::new(FormKind::List(vec![form]), NestedList::span())
        });
        let read = CodeList::of_form(&deep).expect("a list form");
        let wrapped = CodeList::new(vec![Value::List(read)], NestedList::span());
        assert_eq!(wrapped.err(), Some(NestedTooDeep));
    }
}

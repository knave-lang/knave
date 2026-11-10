use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use super::Value;
use super::limit_problems::LimitProblems;
use crate::diagnostic::Diagnostic;
use crate::reader::{Form, FormCount, FormKind, FormNesting};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListTooLarge;

impl ListTooLarge {
    pub fn at(self, span: Span) -> Diagnostic {
        LimitProblems::list_too_large(ListConversion::FORM_LIMIT, span)
    }
}

pub struct ListConversion {
    remaining: usize,
}

impl ListConversion {
    pub const FORM_LIMIT: usize = 131_072;

    fn full() -> Self {
        Self {
            remaining: Self::FORM_LIMIT,
        }
    }

    fn form_of(&mut self, value: &Value) -> Result<Form, ListTooLarge> {
        match value {
            Value::List(list) => self.form_of_list(list),
            Value::Form(form) => {
                self.charge(form)?;
                Ok(form.clone())
            }
            _ => unreachable!("a code list holds only code"),
        }
    }

    fn form_of_list(&mut self, list: &CodeList) -> Result<Form, ListTooLarge> {
        self.remaining = self.remaining.checked_sub(1).ok_or(ListTooLarge)?;
        let forms = self.forms_of(list)?;
        Ok(Form::new(FormKind::List(forms), list.span))
    }

    fn forms_of(&mut self, list: &CodeList) -> Result<Vec<Form>, ListTooLarge> {
        let mut forms = Vec::with_capacity(list.len());
        for index in list.range.indices() {
            forms.push(self.form_of(&list.item(index))?);
        }
        Ok(forms)
    }

    fn charge(&mut self, form: &Form) -> Result<(), ListTooLarge> {
        self.remaining = self.remaining.checked_sub(1).ok_or(ListTooLarge)?;
        form.as_list()
            .unwrap_or_default()
            .iter()
            .try_for_each(|item| self.charge(item))
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

    fn with_size_of_items(values: &[Value]) -> Result<(u32, usize), NestedTooDeep> {
        let (deepest, size) = values
            .iter()
            .fold((0, ListSize::EMPTY), |(deepest, size), value| {
                (
                    deepest.max(Self::of_value(value)),
                    size + ListSize::of_value(value),
                )
            });
        Self::admitted(1 + deepest).map(|nesting| (nesting, size))
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
    range: ItemRange,
    span: Span,
    nesting: u32,
    size: u32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct ItemRange {
    start: u32,
    end: u32,
}

impl ItemRange {
    fn new(start: usize, end: usize) -> Self {
        Self {
            start: Self::stored(start),
            end: Self::stored(end),
        }
    }

    fn stored(index: usize) -> u32 {
        debug_assert!(
            u32::try_from(index).is_ok(),
            "a code list holds fewer than 2^32 values"
        );
        index as u32
    }

    fn start(&self) -> usize {
        self.start as usize
    }

    fn end(&self) -> usize {
        self.end as usize
    }

    fn len(&self) -> usize {
        self.indices().len()
    }

    fn indices(&self) -> Range<usize> {
        self.start()..self.end()
    }
}

pub struct ListSize;

impl ListSize {
    pub const EMPTY: usize = 1;

    pub fn of(list: &CodeList) -> usize {
        list.size as usize
    }

    fn of_value(value: &Value) -> usize {
        match value {
            Value::Form(form) => FormCount::stored(form),
            Value::List(list) => Self::of(list),
            _ => Self::EMPTY,
        }
    }

    fn stored(size: usize) -> u32 {
        u32::try_from(size).unwrap_or(u32::MAX)
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
enum ListItems {
    Read(Form),
    Built(Rc<RefCell<Vec<Value>>>),
}

impl CodeList {
    pub fn new(values: Vec<Value>, span: Span) -> Result<Self, NestedTooDeep> {
        let (nesting, size) = ListNesting::with_size_of_items(&values)?;
        Ok(Self {
            range: ItemRange::new(0, values.len()),
            size: ListSize::stored(size),
            items: ListItems::Built(Rc::new(RefCell::new(values))),
            span,
            nesting,
        })
    }

    #[inline]
    pub fn of_form(form: &Form) -> Option<Self> {
        let length = form.as_list()?.len();
        Some(Self {
            items: ListItems::Read(form.clone()),
            range: ItemRange::new(0, length),
            span: form.span(),
            nesting: FormNesting::of(form),
            size: ListSize::stored(FormCount::stored(form)),
        })
    }

    pub fn len(&self) -> usize {
        self.range.len()
    }

    pub fn is_empty(&self) -> bool {
        self.range.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<Value> {
        (index < self.len()).then(|| self.item(self.range.start() + index))
    }

    pub fn slice(&self, range: Range<usize>, span: Span) -> Option<Self> {
        (range.start <= range.end && range.end <= self.len()).then(|| Self {
            items: self.items.clone(),
            range: ItemRange::new(
                self.range.start() + range.start,
                self.range.start() + range.end,
            ),
            span,
            nesting: self.nesting,
            size: self.size,
        })
    }

    pub fn appended(&self, extra: Vec<Value>, span: Span) -> Result<Self, NestedTooDeep> {
        match self.growable_buffer() {
            Some(buffer) => Self::grown(self, buffer, extra, span),
            None => Self::new([self.values(), extra].concat(), span),
        }
    }

    pub fn append_work(&self, added: usize) -> usize {
        if self.growable_buffer().is_some() {
            added
        } else {
            added + self.len()
        }
    }

    pub fn values(&self) -> Vec<Value> {
        self.range.indices().map(|index| self.item(index)).collect()
    }

    pub fn forms(&self) -> Result<Vec<Form>, ListTooLarge> {
        ListConversion::full().forms_of(self)
    }

    pub fn to_form(&self) -> Result<Form, ListTooLarge> {
        ListConversion::full().form_of_list(self)
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
        for index in self.range.indices() {
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
        let (extra_nesting, extra_size) = ListNesting::with_size_of_items(&extra)?;
        let nesting = list.nesting.max(extra_nesting);
        let size = ListSize::of(list).saturating_add(extra_size - ListSize::EMPTY);
        buffer.borrow_mut().extend(extra);
        Ok(Self {
            range: ItemRange::new(list.range.start(), buffer.borrow().len()),
            items: ListItems::Built(Rc::clone(buffer)),
            span,
            nesting,
            size: ListSize::stored(size),
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
            ListItems::Built(buffer) if buffer.borrow().len() == self.range.end() => Some(buffer),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CodeList, ListConversion, ListNesting, ListSize, ListTooLarge, NestedTooDeep, PassedLimit,
    };
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
    fn a_list_counts_itself_and_each_appearance_of_a_list_it_holds() {
        let inner = NestedList::list_of(vec![NestedList::leaf(), NestedList::leaf()]);
        let outer = NestedList::list_of(vec![Value::List(inner.clone()), Value::List(inner)]);
        assert_eq!(ListSize::of(&outer), 7);
    }

    #[test]
    fn appending_adds_the_size_of_each_new_value() {
        let list = NestedList::list_of(vec![NestedList::leaf()]);
        let longer = list
            .appended(
                vec![NestedList::leaf(), NestedList::leaf()],
                NestedList::span(),
            )
            .expect("within the limit");
        assert_eq!(ListSize::of(&longer), 4);
    }

    #[test]
    fn a_size_past_what_a_list_stores_is_kept_at_the_largest() {
        assert_eq!(ListSize::stored((1 << 32) + 1), u32::MAX);
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
    fn a_list_within_the_form_limit_converts() {
        let items = vec![NestedList::leaf(); ListConversion::FORM_LIMIT - 1];
        assert!(NestedList::list_of(items).to_form().is_ok());
    }

    #[test]
    fn a_list_one_past_the_form_limit_does_not_convert() {
        let items = vec![NestedList::leaf(); ListConversion::FORM_LIMIT];
        assert_eq!(
            NestedList::list_of(items).to_form().err(),
            Some(ListTooLarge)
        );
    }

    #[test]
    fn a_shared_list_is_charged_for_every_copy() {
        let shared = (0..64).fold(NestedList::leaf(), |item, _| {
            Value::List(NestedList::list_of(vec![item.clone(), item]))
        });
        let list = NestedList::list_of(vec![shared]);
        assert_eq!(list.forms().err(), Some(ListTooLarge));
    }

    #[test]
    fn a_shared_form_is_charged_for_every_copy() {
        let leaf = Form::new(FormKind::Symbol(Symbol::new("leaf")), NestedList::span());
        let shared = (0..64).fold(leaf, |item, _| {
            Form::new(FormKind::List(vec![item.clone(), item]), NestedList::span())
        });
        let list = NestedList::list_of(vec![Value::Form(shared)]);
        assert_eq!(list.to_form().err(), Some(ListTooLarge));
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
    fn a_list_at_the_end_of_its_buffer_charges_only_the_added_items() {
        let list = NestedList::list_of(vec![NestedList::leaf(), NestedList::leaf()]);
        assert_eq!(list.append_work(3), 3);
    }

    #[test]
    fn a_list_before_the_end_of_its_buffer_charges_the_copied_items() {
        let list = NestedList::list_of(vec![NestedList::leaf(), NestedList::leaf()]);
        let prefix = list.slice(0..1, NestedList::span()).expect("in range");
        assert_eq!(prefix.append_work(3), 4);
    }

    #[test]
    fn a_read_list_charges_the_copied_items() {
        let read = Form::new(
            FormKind::List(vec![Form::new(
                FormKind::Symbol(Symbol::new("leaf")),
                NestedList::span(),
            )]),
            NestedList::span(),
        );
        assert_eq!(
            CodeList::of_form(&read)
                .expect("a list form")
                .append_work(2),
            3
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
    fn a_read_list_takes_its_size_from_its_form() {
        let leaf = Form::new(FormKind::Symbol(Symbol::new("leaf")), NestedList::span());
        let form = Form::new(FormKind::List(vec![leaf.clone(), leaf]), NestedList::span());
        let read = CodeList::of_form(&form).expect("a list form");
        assert_eq!(ListSize::of(&read), 3);
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

use std::fmt;
use std::rc::Rc;

use crate::scope::{ChainPrefix, Scope, ScopeSet};
use crate::source::Span;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Form {
    kind: Rc<FormKind>,
    span: Span,
    shape: FormShape,
}

pub struct FormNesting;

impl FormNesting {
    pub fn of(form: &Form) -> u32 {
        form.shape.nesting()
    }
}

pub struct FormCount;

impl FormCount {
    pub const LARGEST_STORED: usize = FormShape::LARGEST_FORM_COUNT as usize;

    pub fn stored(form: &Form) -> usize {
        form.shape.stored_form_count()
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
struct FormShape(u32);

impl FormShape {
    const NESTING_BITS: u32 = 13;
    const NESTING_MASK: u32 = (1 << Self::NESTING_BITS) - 1;
    const LARGEST_FORM_COUNT: u32 = u32::MAX >> Self::NESTING_BITS;
    const ONE_FORM: usize = 1;

    fn of(kind: &FormKind) -> Self {
        match kind {
            FormKind::List(items) => Self::of_items(items),
            FormKind::String(_) | FormKind::Symbol(_) | FormKind::Number(_) => {
                Self::new(0, Self::ONE_FORM)
            }
        }
    }

    fn of_items(items: &[Form]) -> Self {
        let (deepest, forms) = items
            .iter()
            .fold((0, Self::ONE_FORM), |(deepest, forms), item| {
                let shape = item.shape;
                (
                    deepest.max(shape.nesting()),
                    forms + shape.stored_form_count(),
                )
            });
        Self::new(deepest + 1, forms)
    }

    fn new(nesting: u32, forms: usize) -> Self {
        let nesting = nesting.min(Self::NESTING_MASK);
        let forms = u32::try_from(forms).map_or(Self::LARGEST_FORM_COUNT, |forms| {
            forms.min(Self::LARGEST_FORM_COUNT)
        });
        Self(forms << Self::NESTING_BITS | nesting)
    }

    fn nesting(self) -> u32 {
        self.0 & Self::NESTING_MASK
    }

    fn form_count(self) -> Option<usize> {
        let forms = self.stored_form_count();
        (forms < Self::LARGEST_FORM_COUNT as usize).then_some(forms)
    }

    fn stored_form_count(self) -> usize {
        (self.0 >> Self::NESTING_BITS) as usize
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum FormKind {
    Symbol(Symbol),
    String(Rc<str>),
    Number(Number),
    List(Vec<Form>),
}

impl Form {
    pub const NESTING_LIMIT: usize = 1024;

    pub fn new(kind: FormKind, span: Span) -> Self {
        let shape = FormShape::of(&kind);
        Self {
            kind: Rc::new(kind),
            span,
            shape,
        }
    }

    pub fn kind(&self) -> &FormKind {
        &self.kind
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn is_same_form(&self, other: &Form) -> bool {
        Rc::ptr_eq(&self.kind, &other.kind)
    }

    pub fn identity(&self) -> *const FormKind {
        Rc::as_ptr(&self.kind)
    }

    pub fn form_count(&self) -> usize {
        self.shape.form_count().unwrap_or_else(|| {
            let items = self.as_list().unwrap_or_default();
            1 + items.iter().map(Form::form_count).sum::<usize>()
        })
    }

    pub fn as_symbol(&self) -> Option<&Symbol> {
        match self.kind() {
            FormKind::Symbol(symbol) => Some(symbol),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Form]> {
        match self.kind() {
            FormKind::List(items) => Some(items),
            _ => None,
        }
    }

    pub fn located_at(self, span: Span) -> Self {
        Self { span, ..self }
    }

    pub fn set_scopes(&mut self, scopes: &ScopeSet) {
        self.update_scopes(&|current| current.clone_from(scopes));
    }

    pub fn add_scope(&mut self, scope: Scope) {
        self.update_scopes(&|scopes| scopes.add(scope));
    }

    pub fn add_prefix(&mut self, prefix: &ChainPrefix) {
        self.update_scopes(&|scopes| scopes.add_prefix(prefix));
    }

    pub fn flip_scope(&mut self, scope: Scope) {
        self.update_scopes(&|scopes| scopes.flip(scope));
    }

    pub fn flip_scope_within(&mut self, scope: Scope, allowance: &mut usize) -> bool {
        self.update_scopes_within(&|scopes| scopes.flip(scope), allowance)
    }

    pub fn remove_scope(&mut self, scope: Scope) {
        self.update_scopes(&|scopes| scopes.remove(scope));
    }

    pub fn retain_scopes(&mut self, keep: &dyn Fn(Scope) -> bool) {
        self.update_scopes(&|scopes| scopes.retain(keep));
    }

    pub fn relocate_outside(&mut self, region: Span) {
        if !region.contains(self.span) && !self.span.is_pinned() {
            self.span = region;
        }
        if self.as_list().is_some()
            && let FormKind::List(items) = Rc::make_mut(&mut self.kind)
        {
            items
                .iter_mut()
                .for_each(|item| item.relocate_outside(region));
        }
    }

    fn update_scopes(&mut self, update: &dyn Fn(&mut ScopeSet)) {
        match self.scoped_kind_mut() {
            Some(FormKind::Symbol(symbol)) => update(symbol.scopes_mut()),
            Some(FormKind::List(items)) => {
                items.iter_mut().for_each(|item| item.update_scopes(update))
            }
            _ => {}
        }
    }

    fn update_scopes_within(
        &mut self,
        update: &dyn Fn(&mut ScopeSet),
        allowance: &mut usize,
    ) -> bool {
        let Some(remaining) = allowance.checked_sub(1) else {
            return false;
        };
        *allowance = remaining;
        match self.scoped_kind_mut() {
            Some(FormKind::Symbol(symbol)) => {
                update(symbol.scopes_mut());
                true
            }
            Some(FormKind::List(items)) => items
                .iter_mut()
                .all(|item| item.update_scopes_within(update, allowance)),
            _ => true,
        }
    }

    #[inline(always)]
    fn scoped_kind_mut(&mut self) -> Option<&mut FormKind> {
        match self.kind() {
            FormKind::Symbol(_) | FormKind::List(_) => Some(Rc::make_mut(&mut self.kind)),
            FormKind::String(_) | FormKind::Number(_) => None,
        }
    }
}

impl fmt::Display for Form {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind() {
            FormKind::Symbol(symbol) => write!(formatter, "{}", symbol.name()),
            FormKind::String(text) => write!(formatter, "{text:?}"),
            FormKind::Number(number) => write!(formatter, "{}", number.text()),
            FormKind::List(items) => Self::fmt_list(items, formatter),
        }
    }
}

impl Form {
    fn fmt_list(items: &[Form], formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("(")?;
        for (index, item) in items.iter().enumerate() {
            if index > 0 {
                formatter.write_str(" ")?;
            }
            write!(formatter, "{item}")?;
        }
        formatter.write_str(")")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Symbol {
    name: Rc<str>,
    scopes: ScopeSet,
}

impl Symbol {
    pub fn new(name: impl Into<Rc<str>>) -> Self {
        Self::with_scopes(name, ScopeSet::default())
    }

    pub fn with_scopes(name: impl Into<Rc<str>>, scopes: ScopeSet) -> Self {
        Self {
            name: name.into(),
            scopes,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn shared_name(&self) -> Rc<str> {
        Rc::clone(&self.name)
    }

    pub fn scopes(&self) -> &ScopeSet {
        &self.scopes
    }

    pub fn scopes_mut(&mut self) -> &mut ScopeSet {
        &mut self.scopes
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum NumberValue {
    Integer(i128),
    Float(f64),
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Number {
    text: Rc<str>,
    value: NumberValue,
}

impl Number {
    pub fn new(text: impl Into<Rc<str>>, value: NumberValue) -> Self {
        Self {
            text: text.into(),
            value,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn value(&self) -> &NumberValue {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::{Form, FormCount, FormKind, FormNesting, FormShape, Symbol};
    use crate::source::{SourceId, Span};

    struct CountedForms;

    impl CountedForms {
        fn span() -> Span {
            Span::new(SourceId::new(0), 0..1)
        }

        fn leaf() -> Form {
            Form::new(FormKind::Symbol(Symbol::new("leaf")), Self::span())
        }

        fn list(items: Vec<Form>) -> Form {
            Form::new(FormKind::List(items), Self::span())
        }
    }

    #[test]
    fn text_counts_one_form_however_long() {
        let text = Form::new(
            FormKind::String("x".repeat(100).into()),
            CountedForms::span(),
        );
        assert_eq!(text.form_count(), 1);
    }

    #[test]
    fn a_list_counts_itself_and_every_form_of_its_tree() {
        let inner = CountedForms::list(vec![CountedForms::leaf(), CountedForms::leaf()]);
        let outer = CountedForms::list(vec![inner.clone(), inner, CountedForms::leaf()]);
        assert_eq!(outer.form_count(), 8);
        assert_eq!(FormNesting::of(&outer), 2);
    }

    #[test]
    fn a_count_past_what_a_form_stores_is_counted_in_full() {
        let leaves = vec![CountedForms::leaf(); FormShape::LARGEST_FORM_COUNT as usize];
        let list = CountedForms::list(leaves);
        assert_eq!(
            list.form_count(),
            FormShape::LARGEST_FORM_COUNT as usize + 1
        );
    }

    #[test]
    fn a_stored_count_stops_at_the_largest() {
        let leaves = vec![CountedForms::leaf(); FormCount::LARGEST_STORED];
        let list = CountedForms::list(leaves);
        assert_eq!(FormCount::stored(&list), FormCount::LARGEST_STORED);
    }

    #[test]
    fn a_count_past_what_a_form_stores_is_kept_at_the_largest() {
        let shape = FormShape::new(3, (1 << 32) + 1);
        assert_eq!(shape.form_count(), None);
        assert_eq!(shape.nesting(), 3);
    }

    #[test]
    fn a_count_that_fits_32_bits_but_not_the_form_is_kept_at_the_largest() {
        let shape = FormShape::new(3, u32::MAX as usize);
        assert_eq!(shape.form_count(), None);
        assert_eq!(shape.nesting(), 3);
    }

    #[test]
    fn a_count_below_the_largest_is_stored() {
        let count = FormShape::LARGEST_FORM_COUNT as usize - 1;
        assert_eq!(FormShape::new(0, count).form_count(), Some(count));
    }

    #[test]
    fn a_nesting_past_what_a_form_stores_is_kept_at_the_largest() {
        let shape = FormShape::new(u32::MAX, 5);
        assert_eq!(shape.nesting(), FormShape::NESTING_MASK);
        assert_eq!(shape.form_count(), Some(5));
    }

    struct RelocationFixture {
        region: Span,
    }

    impl RelocationFixture {
        fn new() -> Self {
            Self {
                region: Span::new(SourceId::new(0), 0..4),
            }
        }

        fn relocated(&self, span: Span) -> Span {
            let mut form = Form::new(FormKind::String("text".into()), span);
            form.relocate_outside(self.region);
            form.span()
        }
    }

    #[test]
    fn a_form_from_another_source_moves_onto_the_region() {
        let fixture = RelocationFixture::new();
        let outside = Span::new(SourceId::new(1), 8..12);
        assert_eq!(fixture.relocated(outside), fixture.region);
    }

    #[test]
    fn a_pinned_form_keeps_its_location() {
        let fixture = RelocationFixture::new();
        let pinned = Span::new(SourceId::new(1).pinned(), 8..12);
        assert_eq!(fixture.relocated(pinned), pinned);
    }
}

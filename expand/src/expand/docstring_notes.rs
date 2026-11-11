use std::rc::Rc;

use crate::interpret::Value;

pub(super) struct DocstringNotes {
    key: Rc<str>,
    documented_terms_key: Rc<str>,
    noted_term: Value,
}

impl DocstringNotes {
    pub(super) const PAGE_MACRO: &'static str = "doc-page";
    pub(super) const DOCSTRINGS_KEY: &'static str = "docstrings-key";
    pub(super) const DOCUMENTED_TERMS_KEY: &'static str = "documented-terms-key";
    pub(super) const NOTED_TERM: &'static str = "noted-term";

    pub(super) fn of(key: &Value, documented_terms_key: &Value, noted_term: Value) -> Option<Self> {
        Some(Self {
            key: Self::symbol_name(key)?,
            documented_terms_key: Self::symbol_name(documented_terms_key)?,
            noted_term,
        })
    }

    pub(super) fn keys(&self) -> [&Rc<str>; 2] {
        [&self.key, &self.documented_terms_key]
    }

    pub(super) fn noted_term(&self) -> &Value {
        &self.noted_term
    }

    fn symbol_name(value: &Value) -> Option<Rc<str>> {
        Some(value.as_form()?.as_symbol()?.shared_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::{Form, FormKind, Symbol};
    use crate::source::{SourceId, Span};

    struct NotesFixture;

    impl NotesFixture {
        const KEY: &str = "docstrings";
        const DOCUMENTED_TERMS_KEY: &str = "documented-terms";
        const ACCESSOR: &str = "accessor";

        fn symbol(name: &str) -> Value {
            let span = Span::new(SourceId::new(0), 0..name.len());
            Value::Form(Form::new(FormKind::Symbol(Symbol::new(name)), span))
        }

        fn notes() -> Option<DocstringNotes> {
            DocstringNotes::of(
                &Self::symbol(Self::KEY),
                &Self::symbol(Self::DOCUMENTED_TERMS_KEY),
                Self::symbol(Self::ACCESSOR),
            )
        }
    }

    #[test]
    fn the_keys_are_the_names_of_both_key_symbols_in_order() {
        let notes = NotesFixture::notes().expect("both keys are symbols");
        let keys: Vec<&str> = notes.keys().into_iter().map(|key| &**key).collect();
        assert_eq!(
            keys,
            [NotesFixture::KEY, NotesFixture::DOCUMENTED_TERMS_KEY]
        );
    }

    #[test]
    fn the_noted_term_is_the_accessor_it_was_given() {
        let notes = NotesFixture::notes().expect("both keys are symbols");
        let accessor = notes.noted_term().as_form().and_then(Form::as_symbol);
        assert_eq!(accessor.map(Symbol::name), Some(NotesFixture::ACCESSOR));
    }

    #[test]
    fn a_key_that_is_not_a_symbol_gives_no_notes() {
        let notes = DocstringNotes::of(
            &Value::Bool(true),
            &NotesFixture::symbol(NotesFixture::DOCUMENTED_TERMS_KEY),
            NotesFixture::symbol(NotesFixture::ACCESSOR),
        );
        assert!(notes.is_none());
    }
}

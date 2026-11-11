use std::rc::Rc;

use crate::interpret::Value;

pub(super) struct DocstringNotes {
    pub key: Rc<str>,
    pub documented_terms_key: Rc<str>,
    pub noted_term: Value,
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

    fn symbol_name(value: &Value) -> Option<Rc<str>> {
        Some(value.as_form()?.as_symbol()?.shared_name())
    }
}

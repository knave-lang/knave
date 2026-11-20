use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::Template;
use crate::core::ParsedTemplate;
use crate::emit::format_arguments::FormatArguments;

#[derive(Default)]
pub struct JudgedTemplates {
    formatted: RefCell<FxHashMap<Rc<str>, Rc<[usize]>>>,
}

impl JudgedTemplates {
    pub fn template(&self, text: &Rc<str>) -> Template {
        let parsed = ParsedTemplate::parse(text).expect("the expansion stage checked the template");
        let formatted = Rc::clone(
            self.formatted
                .borrow_mut()
                .entry(Rc::clone(text))
                .or_insert_with(|| FormatArguments::new(parsed.pieces()).formatted().into()),
        );
        Template {
            parsed,
            formatted: OnceCell::from(formatted),
        }
    }
}

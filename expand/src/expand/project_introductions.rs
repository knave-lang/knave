use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::macro_origin::MacroOrigin;
use crate::reader::Symbol;
use crate::scope::Scope;

#[derive(Default)]
pub struct ProjectIntroductions {
    macros: FxHashMap<Scope, Rc<str>>,
}

impl ProjectIntroductions {
    pub fn record(&mut self, introduction: Scope, origin: &MacroOrigin) {
        if origin.in_project {
            self.macros.insert(introduction, Rc::clone(&origin.name));
        }
    }

    pub fn writer_of(&self, binder: &Symbol) -> Option<Rc<str>> {
        self.macros
            .iter()
            .filter(|(introduction, _)| binder.scopes().contains(**introduction))
            .max_by_key(|(introduction, _)| **introduction)
            .map(|(_, name)| Rc::clone(name))
    }
}

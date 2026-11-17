use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::macro_origin::MacroOrigin;
use crate::reader::Symbol;
use crate::scope::Scope;

#[derive(Default)]
pub struct IntroducingMacros {
    writers: FxHashMap<Scope, TemplateWriter>,
}

struct TemplateWriter {
    name: Rc<str>,
    in_project: bool,
}

impl IntroducingMacros {
    pub fn record(&mut self, introduction: Scope, origin: &MacroOrigin) {
        let writer = TemplateWriter {
            name: Rc::clone(&origin.name),
            in_project: origin.in_project,
        };
        self.writers.insert(introduction, writer);
    }

    pub fn project_writer_of(&self, binder: &Symbol) -> Option<Rc<str>> {
        self.writers
            .iter()
            .filter(|(introduction, _)| binder.scopes().contains(**introduction))
            .min_by_key(|(introduction, _)| **introduction)
            .map(|(_, writer)| writer)
            .filter(|writer| writer.in_project)
            .map(|writer| Rc::clone(&writer.name))
    }
}

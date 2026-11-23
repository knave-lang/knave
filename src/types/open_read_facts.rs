use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::settled_read::SettledRead;
use super::ty::{Type, TypeVar};
use crate::core::FieldLabel;
use crate::source::Span;

#[derive(Default)]
pub(super) struct OpenReadFacts {
    requested: FxHashSet<Rc<str>>,
    settled: Rc<FxHashMap<Span, SettledRead>>,
    opened: Vec<OpenedRead>,
}

pub(super) struct OpenedRead {
    pub(super) owner: TypeVar,
    pub(super) read: Type,
    pub(super) label: FieldLabel,
}

impl OpenReadFacts {
    pub(super) fn new(settled: Rc<FxHashMap<Span, SettledRead>>) -> Self {
        Self {
            settled,
            ..Self::default()
        }
    }

    pub(super) fn request(&mut self, field: &Rc<str>) {
        self.requested.insert(Rc::clone(field));
    }

    pub(super) fn take_requested(&mut self) -> FxHashSet<Rc<str>> {
        std::mem::take(&mut self.requested)
    }

    pub(super) fn open(&mut self, opened: OpenedRead) {
        self.opened.push(opened);
    }

    pub(super) fn take_opened(&mut self) -> Vec<OpenedRead> {
        std::mem::take(&mut self.opened)
    }

    pub(super) fn settled(&self, read: Span) -> Option<&SettledRead> {
        self.settled.get(&read)
    }
}

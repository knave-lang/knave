use std::rc::Rc;

use rustc_hash::FxHashMap;

use crate::core::Namespace;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(super) struct QualifiedNames {
    by_last_segment: FxHashMap<(Rc<str>, Namespace), Vec<Rc<str>>>,
}

impl QualifiedNames {
    const SEPARATOR: char = '/';

    pub(super) fn record(&mut self, (name, namespace): &(Rc<str>, Namespace)) {
        if let Some((_, last_segment)) = name.rsplit_once(Self::SEPARATOR) {
            let names = self
                .by_last_segment
                .entry((last_segment.into(), *namespace))
                .or_default();
            if !names.contains(name) {
                names.push(Rc::clone(name));
            }
        }
    }

    pub(super) fn record_saved(&mut self, name: &str, namespace: Namespace) {
        if name.contains(Self::SEPARATOR) {
            self.record(&(Rc::from(name), namespace));
        }
    }

    pub(super) fn ending_in(&self, last_segment: Rc<str>, namespace: Namespace) -> &[Rc<str>] {
        self.by_last_segment
            .get(&(last_segment, namespace))
            .map_or(&[], Vec::as_slice)
    }
}

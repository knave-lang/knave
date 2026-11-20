use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::ty::Bound;
use crate::core::BindingId;

#[derive(Default)]
pub struct FieldTraitBindings {
    by_field: FxHashMap<Rc<str>, BindingId>,
    by_binding: FxHashMap<BindingId, Rc<str>>,
}

impl FieldTraitBindings {
    pub fn record(&mut self, field: Rc<str>, binding: BindingId) {
        self.by_binding.insert(binding, Rc::clone(&field));
        self.by_field.insert(field, binding);
    }

    pub fn trait_of(&self, field: &str) -> Option<BindingId> {
        self.by_field.get(field).copied()
    }

    pub fn field_of(&self, binding: BindingId) -> Option<&Rc<str>> {
        self.by_binding.get(&binding)
    }

    pub fn is_field_bound(&self, bound: &Bound) -> bool {
        self.by_binding.contains_key(&bound.binding)
    }

    pub fn is_empty(&self) -> bool {
        self.by_field.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Rc<str>, BindingId)> {
        (self.by_field.iter()).map(|(field, binding)| (field, *binding))
    }
}

use std::rc::Rc;

use knave_expand::mangle::NameMangler;

use super::implementor::Implementor;
use super::rust_trait::RustTrait;
use super::trait_rows_reach::LoweredRows;
use crate::core::{BindingArena, BindingId, BindingInfo, BindingKind, Keyword};

pub(super) struct FieldTrait {
    field: Rc<str>,
    implementors: Vec<(BindingId, Implementor)>,
}

impl FieldTrait {
    pub(super) fn new(field: Rc<str>, implementors: Vec<(BindingId, Implementor)>) -> Self {
        Self {
            field,
            implementors,
        }
    }

    pub(super) fn rust_name(field: &str) -> String {
        NameMangler::type_name(&format!("has-{field}"))
    }

    pub(super) fn binding_info(&self, bindings: &BindingArena) -> BindingInfo {
        let module = (self.implementors.first())
            .and_then(|(owner, _)| bindings.get(*owner).kind().defining_module())
            .expect("a field trait has an implementing struct in a module");
        let name = format!("has {}{}", Keyword::MARKER, self.field);
        BindingInfo::new(name.into(), BindingKind::Trait(module), None)
    }

    pub(super) fn into_rust_trait(self) -> (Rc<str>, RustTrait) {
        let implementors = self
            .implementors
            .into_iter()
            .map(|(_, implementor)| implementor);
        let rows = LoweredRows::new(implementors.collect(), Vec::new());
        let path = Self::rust_name(&self.field);
        let read = NameMangler::type_name(&self.field);
        (
            self.field,
            RustTrait::field_read(path.into(), read.into(), rows),
        )
    }
}

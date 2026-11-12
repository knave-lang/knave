use std::rc::Rc;

use super::problems::TypeProblems;
use super::ty::{GenericParam, Type};
use crate::core::TypeBinder;
use crate::diagnostic::Diagnostic;

pub(super) struct FieldParameters<'a> {
    binders: &'a [TypeBinder],
    params: &'a [GenericParam],
}

impl<'a> FieldParameters<'a> {
    pub(super) fn new(binders: &'a [TypeBinder], params: &'a [GenericParam]) -> Self {
        Self { binders, params }
    }

    pub(super) fn unused_in(&self, fields: &[(Rc<str>, Type)]) -> Option<Diagnostic> {
        let (binder, param) = self
            .binders
            .iter()
            .filter(|binder| !binder.lifetime)
            .zip(self.params)
            .find(|(_, param)| !fields.iter().any(|(_, field)| field.mentions(param.id)))?;
        Some(TypeProblems::unused_type_parameter(
            &param.name,
            binder.span,
        ))
    }
}

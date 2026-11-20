use std::rc::Rc;

use super::Expander;
use super::problems::ExpandProblems;
use crate::core::{BindingKind, KernelType, Keyword, Namespace, TypeBound};
use crate::diagnostic::Diagnostic;
use crate::reader::Form;

pub(super) struct FieldBoundForm<'e, 's> {
    expander: &'e mut Expander<'s>,
}

impl<'e, 's> FieldBoundForm<'e, 's> {
    pub(super) fn new(expander: &'e mut Expander<'s>) -> Self {
        Self { expander }
    }

    pub(super) fn lower(self, bound: &Form) -> Option<Result<TypeBound, Diagnostic>> {
        let [name, field, read] = bound.as_list()? else {
            return None;
        };
        let symbol = name.as_symbol()?;
        let binding = self
            .expander
            .resolve_use(symbol, Namespace::Type, name.span())
            .ok()?;
        if !matches!(
            self.expander.kind(binding),
            BindingKind::KernelType(KernelType::Has)
        ) {
            return None;
        }
        let malformed = || ExpandProblems::malformed_type(KernelType::Has, bound.span());
        let field = field
            .as_symbol()
            .and_then(|symbol| Keyword::field_name(symbol.name()));
        let Some(field) = field.map(Rc::<str>::from) else {
            return Some(Err(malformed()));
        };
        Some(self.expander.lower_type(read).map(|read| TypeBound {
            binding,
            argument: Some(read),
            field: Some(field),
        }))
    }
}

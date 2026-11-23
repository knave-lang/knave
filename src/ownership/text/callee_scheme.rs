use std::borrow::Cow;

use super::text_traits::TextTraits;
use crate::types::{GenericId, GenericParam, Type};

pub struct CalleeScheme<'t> {
    declared: Cow<'t, Type>,
    generics: &'t [GenericParam],
}

impl<'t> CalleeScheme<'t> {
    pub fn new(declared: &'t Type, generics: &'t [GenericParam]) -> Self {
        Self {
            declared: Cow::Borrowed(declared),
            generics,
        }
    }

    pub fn written(declared: Type) -> Self {
        Self {
            declared: Cow::Owned(declared),
            generics: &[],
        }
    }

    pub fn signature(&self) -> Option<(&[Type], &Type)> {
        match self.declared.as_ref() {
            Type::Function(_, parameters, result) => Some((parameters, result)),
            _ => None,
        }
    }

    pub fn needs_owned_text(&self, generic: GenericId, traits: TextTraits<'_>) -> bool {
        self.generics
            .iter()
            .filter(|param| param.id == generic)
            .flat_map(|param| &param.bounds)
            .any(|bound| traits.owns_text_for(bound))
    }
}

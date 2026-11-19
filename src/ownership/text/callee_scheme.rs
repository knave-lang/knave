use super::text_program::TextProgram;
use crate::types::{GenericId, GenericParam, Type};

#[derive(Clone, Copy)]
pub struct CalleeScheme<'t> {
    declared: &'t Type,
    generics: &'t [GenericParam],
}

impl<'t> CalleeScheme<'t> {
    pub fn new(declared: &'t Type, generics: &'t [GenericParam]) -> Self {
        Self { declared, generics }
    }

    pub fn signature(&self) -> Option<(&'t [Type], &'t Type)> {
        match self.declared {
            Type::Function(_, parameters, result) => Some((parameters, result)),
            _ => None,
        }
    }

    pub fn needs_owned_text(&self, generic: GenericId, program: &TextProgram<'_>) -> bool {
        self.generics
            .iter()
            .filter(|param| param.id == generic)
            .flat_map(|param| &param.bounds)
            .any(|bound| program.owns_text_for(bound))
    }
}

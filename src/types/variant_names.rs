use std::rc::Rc;

use super::enums::EnumDefinition;
use super::problems::TypeProblems;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub(super) struct VariantNames {
    names: Vec<Rc<str>>,
}

impl VariantNames {
    pub(super) fn of(definition: &EnumDefinition) -> Self {
        let names = definition
            .variants
            .iter()
            .map(|variant| variant.name.clone())
            .collect();
        Self { names }
    }

    pub(super) fn index_of(
        &self,
        name: &str,
        enum_name: &str,
        span: Span,
    ) -> Result<usize, Diagnostic> {
        self.names
            .iter()
            .position(|known| known.as_ref() == name)
            .ok_or_else(|| TypeProblems::unknown_variant(enum_name, name, &self.names, span))
    }
}

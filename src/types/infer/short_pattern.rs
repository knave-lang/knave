use std::rc::Rc;

use crate::core::{BindingId, PatternArguments};
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::EnumDefinition;
use crate::types::problems::TypeProblems;

pub(super) struct ShortPattern<'a> {
    pub(super) name: &'a str,
    pub(super) candidates: &'a [BindingId],
    pub(super) written: &'a PatternArguments,
    pub(super) span: Span,
}

impl ShortPattern<'_> {
    pub(super) fn variant_index(
        &self,
        definition: &EnumDefinition,
        enum_name: &str,
    ) -> Result<usize, Diagnostic> {
        let names: Vec<Rc<str>> = definition
            .variants
            .iter()
            .map(|variant| variant.name.clone())
            .collect();
        names
            .iter()
            .position(|name| name.as_ref() == self.name)
            .ok_or_else(|| TypeProblems::unknown_variant(enum_name, self.name, &names, self.span))
    }
}

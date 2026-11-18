use super::declarations::Declarations;
use super::derive_rules::DeriveRules;
use super::display::TypeDisplay;
use crate::core::BindingArena;

pub(super) struct ExplanationSources<'a> {
    pub(super) declarations: &'a Declarations,
    pub(super) bindings: &'a BindingArena,
    pub(super) display: TypeDisplay<'a>,
    pub(super) derive_rules: DeriveRules<'a>,
}

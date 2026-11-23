use super::typed::TypedDefinition;
use crate::core::ModuleId;

#[derive(Clone, Copy)]
pub struct DefinitionEntry<'p> {
    pub definition: &'p TypedDefinition,
    pub module: ModuleId,
}

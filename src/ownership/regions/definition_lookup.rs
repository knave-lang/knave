use crate::core::BindingId;
use crate::types::TypedDefinition;

pub trait DefinitionLookup {
    fn definition(&self, binding: BindingId) -> Option<&TypedDefinition>;
}

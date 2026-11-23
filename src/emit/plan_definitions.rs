use super::plan::EmitPlan;
use crate::core::BindingId;
use crate::ownership::DefinitionLookup;
use crate::types::TypedDefinition;

impl DefinitionLookup for EmitPlan<'_> {
    fn definition(&self, binding: BindingId) -> Option<&TypedDefinition> {
        EmitPlan::definition(self, binding)
    }
}

use super::{ConstructorCatalog, MatchCoverage, WitnessText};
use crate::core::BindingId;
use crate::types::Type;
use crate::types::checker::TypeChecker;
use crate::types::peeling::ReferencePeeling;
use crate::types::variant_payloads::VariantPayloads;

impl TypeChecker {
    pub(in crate::types) fn match_coverage(&self) -> MatchCoverage<'_> {
        let catalog = ConstructorCatalog::new(
            ReferencePeeling::new(&self.unifier),
            &self.structs,
            self.variant_payloads(),
        );
        let text = WitnessText::new(&self.bindings, &self.enums);
        MatchCoverage::new(&self.bindings, catalog, text)
    }

    pub(in crate::types) fn visible_payloads(
        &self,
        owner: BindingId,
        index: usize,
        arguments: &[Type],
    ) -> Vec<Type> {
        self.variant_payloads().visible(owner, index, arguments)
    }

    pub(in crate::types) fn is_inferred_box(
        &self,
        owner: BindingId,
        variant: usize,
        payload: usize,
    ) -> bool {
        self.variant_payloads()
            .is_inferred_box(owner, variant, payload)
    }

    pub(in crate::types) fn unboxed(&self, ty: &Type) -> Option<Type> {
        self.variant_payloads().unboxed(ty)
    }

    fn variant_payloads(&self) -> VariantPayloads<'_> {
        VariantPayloads::new(&self.enums, self.box_binding(), &self.inferred_boxes)
    }
}

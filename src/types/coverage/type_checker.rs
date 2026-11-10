use super::{ConstructorCatalog, MatchCoverage, WitnessText};
use crate::types::checker::TypeChecker;
use crate::types::peeling::ReferencePeeling;

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
}

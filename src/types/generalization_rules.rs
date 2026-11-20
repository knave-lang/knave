use std::rc::Rc;

use rustc_hash::FxHashSet;

use super::literal_parameters::LiteralParameters;
use super::trait_inclusions::TraitInclusions;
use super::ty::{Bound, TypeVar};

pub(super) struct GeneralizationRules {
    pinnable: FxHashSet<TypeVar>,
    literal_parameters: LiteralParameters,
    inclusions: Rc<TraitInclusions>,
}

impl GeneralizationRules {
    pub(super) fn new(
        pinnable: FxHashSet<TypeVar>,
        literal_parameters: LiteralParameters,
        inclusions: Rc<TraitInclusions>,
    ) -> Self {
        Self {
            pinnable,
            literal_parameters,
            inclusions,
        }
    }

    pub(super) fn generic_bounds(&self, bounds: Vec<Bound>) -> Vec<Bound> {
        self.inclusions.generic_bounds(bounds)
    }

    pub(super) fn defaults_literal(&self, var: TypeVar) -> bool {
        self.literal_parameters == LiteralParameters::Default || !self.pinnable.contains(&var)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::generalization_fixture::GeneralizationFixture;
    use crate::types::literal_parameters::LiteralParameters;

    #[test]
    fn a_pinnable_literal_is_defaulted_when_literal_parameters_default() {
        let mut fixture = GeneralizationFixture::new();
        let var = fixture.literal_var();
        let rules = GeneralizationFixture::rules(&[var], LiteralParameters::Default);
        assert!(rules.defaults_literal(var));
    }

    #[test]
    fn a_pinnable_literal_stays_open_when_literal_parameters_keep_open() {
        let mut fixture = GeneralizationFixture::new();
        let var = fixture.literal_var();
        let rules = GeneralizationFixture::rules(&[var], LiteralParameters::KeepOpen);
        assert!(!rules.defaults_literal(var));
    }

    #[test]
    fn a_literal_nothing_pins_is_defaulted() {
        let mut fixture = GeneralizationFixture::new();
        let var = fixture.literal_var();
        let rules = GeneralizationFixture::rules(&[], LiteralParameters::KeepOpen);
        assert!(rules.defaults_literal(var));
    }
}

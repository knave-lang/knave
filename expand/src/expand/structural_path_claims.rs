use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::structural_path_claim::StructuralPathClaim;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct StructuralPathClaims {
    claims: FxHashMap<Rc<str>, StructuralPathClaim>,
}

impl StructuralPathClaims {
    pub fn claim(
        &mut self,
        path: &str,
        claim: StructuralPathClaim,
    ) -> Result<(), StructuralPathClaim> {
        match self.claims.get(path) {
            Some(first) => Err(*first),
            None => {
                self.claims.insert(path.into(), claim);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{StructuralPathClaim, StructuralPathClaims};
    use crate::core::{BindingArena, BindingId, BindingInfo, BindingKind};
    use crate::source::{SourceId, Span};

    struct ClaimsFixture {
        traits: BindingArena,
        claims: StructuralPathClaims,
    }

    impl ClaimsFixture {
        const PATH: &str = "std::fmt::Display";

        fn new() -> Self {
            Self {
                traits: BindingArena::default(),
                claims: StructuralPathClaims::default(),
            }
        }

        fn declare(&mut self, name: &str) -> BindingId {
            self.traits
                .add(BindingInfo::new(name.into(), BindingKind::Local, None))
        }

        fn claim(&mut self, path: &str, owner: BindingId) -> Result<(), BindingId> {
            let clause = Span::new(SourceId::new(0), 0..1);
            self.claims
                .claim(path, StructuralPathClaim::new(owner, clause))
                .map_err(StructuralPathClaim::owner)
        }
    }

    #[test]
    fn the_first_trait_to_claim_a_path_holds_it() {
        let mut fixture = ClaimsFixture::new();
        let display = fixture.declare("display");
        assert_eq!(fixture.claim(ClaimsFixture::PATH, display), Ok(()));
    }

    #[test]
    fn a_second_claim_of_a_path_answers_with_the_first() {
        let mut fixture = ClaimsFixture::new();
        let display = fixture.declare("display");
        let shown = fixture.declare("shown");
        let _ = fixture.claim(ClaimsFixture::PATH, display);
        assert_eq!(fixture.claim(ClaimsFixture::PATH, shown), Err(display));
    }

    #[test]
    fn claims_of_different_paths_do_not_meet() {
        let mut fixture = ClaimsFixture::new();
        let display = fixture.declare("display");
        let hash = fixture.declare("hash");
        let _ = fixture.claim(ClaimsFixture::PATH, display);
        assert_eq!(fixture.claim("std::hash::Hash", hash), Ok(()));
    }
}

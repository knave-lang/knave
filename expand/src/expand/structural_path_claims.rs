use std::rc::Rc;

use rustc_hash::FxHashMap;

use crate::core::BindingId;
use crate::source::Span;

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct StructuralPathClaim {
    owner: BindingId,
    clause: Span,
}

impl StructuralPathClaim {
    pub fn new(owner: BindingId, clause: Span) -> Self {
        Self { owner, clause }
    }

    pub fn owner(self) -> BindingId {
        self.owner
    }

    pub fn clause(self) -> Span {
        self.clause
    }
}

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

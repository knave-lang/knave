use rustc_hash::FxHashSet;

use super::declaration_key::DeclarationKey;
use crate::core::BindingId;

pub struct KeyedCandidate {
    pub binding: BindingId,
    pub key: DeclarationKey,
    pub tried: bool,
}

pub struct KeyedCandidates {
    candidates: Vec<KeyedCandidate>,
}

impl KeyedCandidates {
    pub fn new(candidates: Vec<KeyedCandidate>) -> Self {
        Self { candidates }
    }

    pub fn keys_to_try(programs: &[KeyedCandidates]) -> Vec<DeclarationKey> {
        let mut keys = Vec::new();
        for candidate in programs.iter().flat_map(|program| &program.candidates) {
            if candidate.tried && !keys.contains(&candidate.key) {
                keys.push(candidate.key.clone());
            }
        }
        keys
    }

    pub fn tried_among(&self, tried: &[DeclarationKey]) -> Vec<BindingId> {
        self.candidates
            .iter()
            .filter(|candidate| tried.contains(&candidate.key))
            .map(|candidate| candidate.binding)
            .collect()
    }

    pub fn refuses(&self, removable: &[BindingId], key: &DeclarationKey) -> bool {
        self.candidates
            .iter()
            .any(|candidate| &candidate.key == key && !removable.contains(&candidate.binding))
    }

    pub fn removes_any_but(&self, removable: &[BindingId], kept: &[DeclarationKey]) -> bool {
        self.candidates.iter().any(|candidate| {
            removable.contains(&candidate.binding) && !kept.contains(&candidate.key)
        })
    }

    pub fn first_warned(
        &self,
        removable: &[DeclarationKey],
        warned: &mut FxHashSet<DeclarationKey>,
    ) -> Vec<BindingId> {
        self.candidates
            .iter()
            .filter(|candidate| {
                removable.contains(&candidate.key) && warned.insert(candidate.key.clone())
            })
            .map(|candidate| candidate.binding)
            .collect()
    }
}

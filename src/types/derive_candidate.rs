use super::definition_kind::DefinitionKind;
use super::derive_source::DeriveSource;
use crate::core::BindingId;

#[derive(Clone, Copy)]
pub(super) struct DeriveCandidate {
    pub(super) binding: BindingId,
    pub(super) kind: DefinitionKind,
    pub(super) trait_binding: BindingId,
    pub(super) source: DeriveSource,
}

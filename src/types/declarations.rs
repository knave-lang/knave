use super::trait_declarations::TraitDeclarations;
use super::type_declarations::TypeDeclarations;

#[derive(Default)]
pub(super) struct Declarations {
    pub(super) types: TypeDeclarations,
    pub(super) traits: TraitDeclarations,
}

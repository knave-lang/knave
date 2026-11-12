use super::trait_declarations::TraitDeclarations;
use super::type_declarations::TypeDeclarations;

#[derive(Default)]
pub struct Declarations {
    pub types: TypeDeclarations,
    pub traits: TraitDeclarations,
}

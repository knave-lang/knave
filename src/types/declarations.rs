use super::trait_declarations::TraitDeclarations;
use super::type_declarations::TypeDeclarations;

#[derive(Default)]
pub struct Declarations {
    pub types: TypeDeclarations,
    pub traits: TraitDeclarations,
}

#[cfg(test)]
impl Declarations {
    pub fn with_traits(table: super::TraitTable) -> Self {
        Self {
            traits: TraitDeclarations {
                table,
                ..TraitDeclarations::default()
            },
            ..Self::default()
        }
    }
}

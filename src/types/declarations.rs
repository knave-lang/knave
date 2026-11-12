use super::trait_declarations::TraitDeclarations;
use super::type_declarations::TypeDeclarations;

#[derive(Default)]
pub struct Declarations {
    pub(super) types: TypeDeclarations,
    pub(super) traits: TraitDeclarations,
}

impl Declarations {
    pub fn types(&self) -> &TypeDeclarations {
        &self.types
    }

    pub fn traits(&self) -> &TraitDeclarations {
        &self.traits
    }
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

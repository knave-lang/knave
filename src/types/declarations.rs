use super::signature_declarations::SignatureDeclarations;
use super::trait_declarations::TraitDeclarations;
use super::type_declarations::TypeDeclarations;

#[derive(Default)]
pub struct Declarations {
    pub types: TypeDeclarations,
    pub traits: TraitDeclarations,
    pub signatures: SignatureDeclarations,
}

impl Declarations {
    pub(super) fn without_checking_facts(mut self) -> Self {
        std::mem::take(&mut self.traits.arguments);
        std::mem::take(&mut self.traits.selfless_methods);
        std::mem::take(&mut self.signatures);
        self
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

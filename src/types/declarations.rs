use super::declarations_builder::DeclarationsBuilder;
use super::trait_declarations::TraitDeclarations;
use super::ty::{GenericParam, Scheme};
use super::type_declarations::TypeDeclarations;
use super::written_bodies::WrittenBodies;

#[derive(Default)]
pub struct Declarations {
    types: TypeDeclarations,
    traits: TraitDeclarations,
    written: WrittenBodies,
    frozen: bool,
}

impl Declarations {
    pub fn types(&self) -> &TypeDeclarations {
        &self.types
    }

    pub fn traits(&self) -> &TraitDeclarations {
        &self.traits
    }

    pub fn written_bodies(&self) -> &WrittenBodies {
        &self.written
    }

    pub(super) fn builder(&mut self) -> DeclarationsBuilder<'_> {
        debug_assert!(
            !self.frozen,
            "declarations are built only during collection"
        );
        DeclarationsBuilder::new(&mut self.types, &mut self.traits, &mut self.written)
    }

    pub(super) fn freeze(&mut self) {
        self.frozen = true;
    }

    pub(super) fn record_impl_params(&mut self, position: usize, params: Vec<GenericParam>) {
        let implementation = &mut self.traits.impls[position];
        implementation.params = params.clone();
        let implementor = implementation.implementor.clone();
        self.traits.table.set_implementor_scheme(
            implementation.trait_binding,
            &implementor,
            Scheme {
                lifetimes: Vec::new(),
                params,
                body: implementor.clone(),
            },
        );
    }
}

#[cfg(test)]
impl Declarations {
    pub fn thaw(&mut self) {
        self.frozen = false;
    }

    pub fn types_mut(&mut self) -> &mut TypeDeclarations {
        &mut self.types
    }

    pub fn traits_mut(&mut self) -> &mut TraitDeclarations {
        &mut self.traits
    }

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

#[cfg(test)]
mod tests {
    use super::Declarations;
    use crate::types::TypedSource;

    #[test]
    #[should_panic(expected = "declarations are built only during collection")]
    fn frozen_declarations_refuse_a_builder() {
        let mut declarations = Declarations::default();
        declarations.freeze();
        let _ = declarations.builder();
    }

    #[test]
    #[should_panic(expected = "declarations are built only during collection")]
    fn checking_a_program_freezes_its_declarations() {
        let mut program = TypedSource::expand("(defn main () (do))").typed();
        let _ = program.declarations.builder();
    }

    #[test]
    fn open_declarations_hand_out_a_builder() {
        let mut declarations = Declarations::default();
        declarations
            .builder()
            .record_written_bodies(Default::default());
    }
}

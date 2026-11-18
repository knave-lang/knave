use super::derives_builder::DerivesBuilder;
use super::projections_builder::ProjectionsBuilder;
use super::trait_declarations::TraitDeclarations;
use super::trait_declarations_builder::TraitDeclarationsBuilder;
use super::type_declarations::TypeDeclarations;
use super::type_declarations_builder::TypeDeclarationsBuilder;
use super::written_bodies::WrittenBodies;

pub(super) struct DeclarationsBuilder<'a> {
    types: &'a mut TypeDeclarations,
    traits: &'a mut TraitDeclarations,
    written: &'a mut WrittenBodies,
}

impl<'a> DeclarationsBuilder<'a> {
    pub(super) fn new(
        types: &'a mut TypeDeclarations,
        traits: &'a mut TraitDeclarations,
        written: &'a mut WrittenBodies,
    ) -> Self {
        Self {
            types,
            traits,
            written,
        }
    }

    pub(super) fn types(self) -> TypeDeclarationsBuilder<'a> {
        TypeDeclarationsBuilder::new(self.types)
    }

    pub(super) fn traits(self) -> TraitDeclarationsBuilder<'a> {
        TraitDeclarationsBuilder::new(self.traits)
    }

    pub(super) fn projections(self) -> ProjectionsBuilder<'a> {
        ProjectionsBuilder::new(&mut self.traits.table)
    }

    pub(super) fn derives(self) -> DerivesBuilder<'a> {
        DerivesBuilder::new(self.types, self.traits)
    }

    pub(super) fn record_written_bodies(self, written: WrittenBodies) {
        *self.written = written;
    }
}

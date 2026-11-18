use super::trait_declarations::TraitDeclarations;
use super::type_declarations::TypeDeclarations;
use super::written_bodies::WrittenBodies;

#[derive(Default)]
pub struct Declarations {
    types: TypeDeclarations,
    traits: TraitDeclarations,
    written: WrittenBodies,
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

    pub(super) fn types_mut(&mut self) -> &mut TypeDeclarations {
        &mut self.types
    }

    pub(super) fn traits_mut(&mut self) -> &mut TraitDeclarations {
        &mut self.traits
    }

    pub(super) fn record_written_bodies(&mut self, written: WrittenBodies) {
        self.written = written;
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

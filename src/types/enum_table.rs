use rustc_hash::FxHashMap;

use super::declared_variant::DeclaredVariant;
use super::enums::EnumDefinition;
use super::payload_lowering::PayloadLowering;
use super::undeclared_variant::UndeclaredVariant;
use super::unlowered_variant::UnloweredVariant;
use super::variant_shape::VariantShape;
use crate::core::{BindingId, VariantInfo};

#[derive(Clone, Default)]
pub struct EnumTable {
    enums: FxHashMap<BindingId, EnumDefinition>,
}

impl EnumTable {
    pub fn add(&mut self, binding: BindingId, definition: EnumDefinition) {
        self.enums.insert(binding, definition);
    }

    pub fn get(&self, binding: BindingId) -> Option<&EnumDefinition> {
        self.enums.get(&binding)
    }

    pub(super) fn is_rust(&self, binding: BindingId) -> bool {
        self.get(binding)
            .is_some_and(|definition| definition.rust.is_some())
    }

    pub fn get_mut(&mut self, binding: BindingId) -> Option<&mut EnumDefinition> {
        self.enums.get_mut(&binding)
    }

    pub(super) fn variant_shape(&self, info: VariantInfo) -> Box<dyn VariantShape + '_> {
        let Some(definition) = self.get(info.owner) else {
            return Box::new(UndeclaredVariant::new(info.arity));
        };
        let variant = &definition.variants[info.index];
        match variant.lowering {
            PayloadLowering::Failed => Box::new(UnloweredVariant::new(variant)),
            PayloadLowering::Complete => Box::new(DeclaredVariant::new(definition, info.index)),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (BindingId, &EnumDefinition)> {
        self.enums
            .iter()
            .map(|(binding, definition)| (*binding, definition))
    }
}

#[cfg(test)]
mod tests {
    use crate::types::declarations_fixture::DeclarationsFixture;

    const SOURCE: &str = "
        (struct point)
        (enum level (rust \"Level\") low high)
        (enum mood calm angry)
        (defn main () (do))";

    #[test]
    fn only_an_enum_backed_by_rust_is_a_rust_enum() {
        let declarations = DeclarationsFixture::new(SOURCE);
        let enums = declarations.types().enums();
        assert!(enums.is_rust(declarations.type_named("level")));
        assert!(!enums.is_rust(declarations.type_named("mood")));
        assert!(!enums.is_rust(declarations.type_named("point")));
    }
}

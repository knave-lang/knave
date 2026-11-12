use rustc_hash::{FxHashMap, FxHashSet};

use super::declared_variant::DeclaredVariant;
use super::enums::EnumDefinition;
use super::undeclared_variant::UndeclaredVariant;
use super::unlowered_variant::UnloweredVariant;
use super::variant_shape::VariantShape;
use crate::core::{BindingId, VariantInfo};

#[derive(Clone, Default)]
pub struct EnumTable {
    enums: FxHashMap<BindingId, EnumDefinition>,
    unlowered: FxHashSet<(BindingId, usize)>,
}

impl EnumTable {
    pub fn add(&mut self, binding: BindingId, definition: EnumDefinition) {
        self.enums.insert(binding, definition);
    }

    pub(super) fn mark_unlowered(&mut self, binding: BindingId, variants: Vec<usize>) {
        self.unlowered
            .extend(variants.into_iter().map(|index| (binding, index)));
    }

    pub fn get(&self, binding: BindingId) -> Option<&EnumDefinition> {
        self.enums.get(&binding)
    }

    pub fn get_mut(&mut self, binding: BindingId) -> Option<&mut EnumDefinition> {
        self.enums.get_mut(&binding)
    }

    pub(super) fn variant_shape(&self, info: VariantInfo) -> Box<dyn VariantShape + '_> {
        let Some(definition) = self.get(info.owner) else {
            return Box::new(UndeclaredVariant::new(info.arity));
        };
        if self.unlowered.contains(&(info.owner, info.index)) {
            return Box::new(UnloweredVariant::new(&definition.variants[info.index]));
        }
        Box::new(DeclaredVariant::new(definition, info.index))
    }

    pub fn iter(&self) -> impl Iterator<Item = (BindingId, &EnumDefinition)> {
        self.enums
            .iter()
            .map(|(binding, definition)| (*binding, definition))
    }
}

use rustc_hash::FxHashSet;

use super::checking_facts::CheckingFacts;
use super::collected_impl::CollectedImpl;
use super::collected_rust_trait::CollectedRustTrait;
use super::collected_trait::CollectedTrait;
use super::field_traits::FieldTrait;
use super::rust_trait::RustTrait;
use super::structural_trait::StructuralTrait;
use super::trait_declarations::TraitDeclarations;
use super::trait_rows_reach::LoweredRows;
use crate::core::{BindingArena, BindingId, BindingInfo, BindingKind, Keyword};
use knave_expand::mangle::NameMangler;

pub(super) struct TraitDeclarationsBuilder<'a> {
    traits: &'a mut TraitDeclarations,
}

impl<'a> TraitDeclarationsBuilder<'a> {
    pub(super) fn new(traits: &'a mut TraitDeclarations) -> Self {
        Self { traits }
    }

    pub(super) fn enter_trait(
        self,
        collected: CollectedTrait,
        facts: &mut CheckingFacts,
        synthetic: &mut FxHashSet<BindingId>,
    ) {
        let traits = self.traits;
        traits
            .table
            .add(collected.definition.binding, collected.rust_trait);
        for (method, selfless) in collected.selfless {
            facts.selfless_methods.insert(method, selfless);
        }
        for method in &collected.definition.methods {
            synthetic.extend(std::iter::once(method.binding).chain(method.default));
            let info = collected.definition.method_info(method);
            traits.methods.insert(method.binding, info);
        }
        traits.definitions.push(collected.definition);
    }

    pub(super) fn enter_field_traits(
        self,
        field_traits: Vec<FieldTrait>,
        bindings: &mut BindingArena,
    ) {
        for field_trait in field_traits {
            let module = field_trait
                .implementors
                .first()
                .and_then(|(owner, _)| bindings.get(*owner).kind().defining_module())
                .expect("a field trait has an implementing struct in a module");
            let name = format!("has {}{}", Keyword::MARKER, field_trait.field);
            let binding = bindings.add(BindingInfo::new(
                name.into(),
                BindingKind::Trait(module),
                None,
            ));
            let implementors = field_trait
                .implementors
                .into_iter()
                .map(|(_, implementor)| implementor);
            let rows = LoweredRows::new(implementors.collect(), Vec::new());
            let path = NameMangler::type_name(&format!("has-{}", field_trait.field));
            let read = NameMangler::type_name(&field_trait.field);
            let field_trait_rows = RustTrait::field_read(path.into(), read.into(), rows);
            self.traits.table.add(binding, field_trait_rows);
            self.traits
                .rust_paths
                .field_traits
                .record(field_trait.field, binding);
        }
    }

    pub(super) fn enter_rust_trait(
        self,
        collected: CollectedRustTrait,
        synthetic: &mut FxHashSet<BindingId>,
    ) {
        let traits = self.traits;
        traits
            .table
            .add(collected.declaration.binding, collected.rust_trait);
        if let Some(included) = collected.declaration.relations.included {
            traits
                .table
                .include(collected.declaration.binding, included);
        }
        traits.rust_paths.record(collected.declaration);
        for (method, info) in collected.methods {
            synthetic.insert(method);
            traits.methods.insert(method, info);
        }
    }

    pub(super) fn enter_impl(
        self,
        collected: CollectedImpl,
        synthetic: &mut FxHashSet<BindingId>,
        impl_methods: &mut FxHashSet<BindingId>,
    ) {
        let traits = self.traits;
        if let Some(rows) = traits
            .table
            .reached_rows_mut(collected.definition.trait_binding)
        {
            rows.add(collected.implementor);
        }
        for method in &collected.definition.methods {
            synthetic.insert(method.binding);
            impl_methods.insert(method.binding);
        }
        traits.impls.push(collected.definition);
    }

    pub(super) fn add_structural(self, structural: StructuralTrait) {
        self.traits.structural.push(structural);
    }
}

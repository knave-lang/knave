use rustc_hash::FxHashSet;

use super::emitted_types::EmittedTypes;
use super::field_trait_path::FieldTraitPath;
use super::fragment::Fragment;
use super::item_lifetimes::ItemLifetimes;
use super::module_tree::ModuleTree;
use super::plan::EmitPlan;
use super::types::{LifetimeScope, RustTypes, TypePosition};
use crate::core::{BindingId, ModuleId};
use crate::types::{FieldGetterName, FieldTraitPlan, FieldTraits, Type, TypedProgram};

pub struct FieldTraitItems<'a> {
    program: &'a TypedProgram,
    plan: &'a EmitPlan<'a>,
    types: EmittedTypes<'a>,
}

struct FieldTrait<'p> {
    name: String,
    path: String,
    getter: String,
    plan: &'p FieldTraitPlan,
}

impl<'a> FieldTraitItems<'a> {
    const CRATE_ITEMS: &'static str = "use super::*;";
    const REFERENCES: [&'static str; 2] = ["&", "&mut "];

    pub fn new(program: &'a TypedProgram, plan: &'a EmitPlan<'a>, types: EmittedTypes<'a>) -> Self {
        Self {
            program,
            plan,
            types,
        }
    }

    pub fn of_module(&self, module: ModuleId) -> Vec<Vec<Fragment>> {
        if module != self.program.entry {
            return Vec::new();
        }
        let types = RustTypes::new(self.program, self.plan, self.types, module);
        let field_traits = self
            .program
            .declarations
            .traits()
            .rust_paths()
            .field_traits();
        let traits: Vec<FieldTrait> = (field_traits.plans())
            .filter_map(|(binding, plan)| self.field_trait(binding, plan))
            .collect();
        if traits.is_empty() {
            return Vec::new();
        }
        let implementations = (traits.iter()).flat_map(|field_trait| {
            (field_trait.plan.implementors.iter())
                .map(|implementor| self.implementation(&types, field_trait, implementor))
        });
        std::iter::once(self.trait_module(&types, &traits))
            .chain(implementations)
            .collect()
    }

    fn trait_module(&self, types: &RustTypes, traits: &[FieldTrait]) -> Vec<Fragment> {
        let mut inner = vec![Fragment::Newline];
        let crate_types = self.crate_types();
        let mut reads = traits.iter().map(|field_trait| &field_trait.plan.read);
        if reads.any(|read| Self::reads_crate_type(read, &crate_types)) {
            inner.extend([
                Fragment::Text(Self::CRATE_ITEMS.to_string()),
                Fragment::Newline,
                Fragment::Newline,
            ]);
        }
        let declarations = traits.iter().flat_map(|field_trait| {
            let declaration = self.declaration(types, field_trait);
            let references = (Self::REFERENCES.iter())
                .map(|reference| Self::reference_impl(types, field_trait, reference));
            std::iter::once(declaration).chain(references)
        });
        inner.extend(ModuleTree::separated(declarations.collect()));
        let visibility = FieldTraitPath::new(self.program).visibility();
        vec![
            Fragment::Text(format!("{visibility}mod {} {{", FieldTraits::MODULE)),
            Fragment::Indented(inner),
            Fragment::Newline,
            Fragment::Text("}".to_string()),
        ]
    }

    fn crate_types(&self) -> FxHashSet<BindingId> {
        let modules = self.program.modules.iter();
        let types = modules.flat_map(|module| module.structs.iter().chain(&module.enums));
        types.copied().collect()
    }

    fn reads_crate_type(read: &Type, crate_types: &FxHashSet<BindingId>) -> bool {
        match read {
            Type::Named(binding, _) if crate_types.contains(binding) => true,
            _ => (read.parts().into_iter()).any(|part| Self::reads_crate_type(part, crate_types)),
        }
    }

    fn field_trait<'p>(
        &self,
        binding: BindingId,
        plan: &'p FieldTraitPlan,
    ) -> Option<FieldTrait<'p>> {
        let traits = self.program.declarations.traits();
        let name = traits.table().get(binding)?.path.to_string();
        let field = traits
            .rust_paths()
            .field_traits()
            .bindings()
            .field_of(binding)?;
        Some(FieldTrait {
            name,
            path: FieldTraitPath::new(self.program).of(binding, self.program.entry)?,
            getter: FieldGetterName::of(field),
            plan,
        })
    }

    fn declaration(&self, types: &RustTypes, field_trait: &FieldTrait) -> Vec<Fragment> {
        let read = Self::trait_read(types, field_trait.plan);
        vec![
            Fragment::Text(format!("pub trait {} {{", field_trait.name)),
            Fragment::Indented(vec![Fragment::Text(format!(
                "fn {}(&self) -> {read};",
                field_trait.getter
            ))]),
            Fragment::Newline,
            Fragment::Text("}".to_string()),
        ]
    }

    fn reference_impl(
        types: &RustTypes,
        field_trait: &FieldTrait,
        reference: &str,
    ) -> Vec<Fragment> {
        let read = Self::trait_read(types, field_trait.plan);
        let (name, getter) = (&field_trait.name, &field_trait.getter);
        vec![
            Fragment::Text(format!(
                "impl<T: {name} + ?Sized> {name} for {reference}T {{"
            )),
            Fragment::Indented(vec![
                Fragment::Text(format!("fn {getter}(&self) -> {read} {{")),
                Fragment::Indented(vec![Fragment::Text(format!("T::{getter}(self)"))]),
                Fragment::Newline,
                Fragment::Text("}".to_string()),
            ]),
            Fragment::Newline,
            Fragment::Text("}".to_string()),
        ]
    }

    fn implementation(
        &self,
        types: &RustTypes,
        field_trait: &FieldTrait,
        implementor: &Type,
    ) -> Vec<Fragment> {
        let lifetime = ItemLifetimes::new(self.program).of_implementor(implementor);
        let scope = LifetimeScope::default().with_elided(lifetime.clone());
        let implementor_name = types.show_in(implementor, TypePosition::Signature, &scope);
        let read = Self::trait_read(types, field_trait.plan);
        let impl_generics = lifetime.map(|name| format!("<{name}>")).unwrap_or_default();
        let borrow = field_trait.plan.kind.getter_prefix();
        vec![
            Fragment::Text(format!(
                "impl{impl_generics} {} for {implementor_name} {{",
                field_trait.path
            )),
            Fragment::Indented(vec![
                Fragment::Text(format!("fn {}(&self) -> {read} {{", field_trait.getter)),
                Fragment::Indented(vec![Fragment::Text(format!(
                    "{borrow}self.{}",
                    field_trait.getter
                ))]),
                Fragment::Newline,
                Fragment::Text("}".to_string()),
            ]),
            Fragment::Newline,
            Fragment::Text("}".to_string()),
        ]
    }

    fn trait_read(types: &RustTypes, plan: &FieldTraitPlan) -> String {
        types.show_in(
            &plan.read,
            TypePosition::Signature,
            &LifetimeScope::default(),
        )
    }
}

use knave_expand::mangle::NameMangler;

use super::emitted_types::EmittedTypes;
use super::field_trait_path::FieldTraitPath;
use super::fragment::Fragment;
use super::item_lifetimes::ItemLifetimes;
use super::plan::EmitPlan;
use super::types::{LifetimeScope, RustTypes, TypePosition};
use crate::core::{BindingId, ModuleId};
use crate::types::{FieldTraitPlan, Type, TypedProgram};

pub struct FieldTraitItems<'a> {
    program: &'a TypedProgram,
    plan: &'a EmitPlan<'a>,
    types: EmittedTypes<'a>,
}

struct FieldTrait<'p> {
    binding: BindingId,
    path: String,
    getter: String,
    plan: &'p FieldTraitPlan,
}

impl<'a> FieldTraitItems<'a> {
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
        (field_traits.plans())
            .filter_map(|(binding, plan)| self.field_trait(binding, plan))
            .flat_map(|field_trait| self.items(&types, &field_trait))
            .collect()
    }

    fn field_trait<'p>(
        &self,
        binding: BindingId,
        plan: &'p FieldTraitPlan,
    ) -> Option<FieldTrait<'p>> {
        let traits = self.program.declarations.traits();
        let path = traits.table().get(binding)?.path.to_string();
        let field = traits.rust_paths().field_traits().field_of(binding)?;
        Some(FieldTrait {
            binding,
            path,
            getter: NameMangler::value(field),
            plan,
        })
    }

    fn items(&self, types: &RustTypes, field_trait: &FieldTrait) -> Vec<Vec<Fragment>> {
        let declaration = self.declaration(types, field_trait);
        let implementations = (field_trait.plan.implementors.iter())
            .map(|implementor| self.implementation(types, field_trait, implementor));
        std::iter::once(declaration)
            .chain(implementations)
            .collect()
    }

    fn declaration(&self, types: &RustTypes, field_trait: &FieldTrait) -> Vec<Fragment> {
        let read = Self::trait_read(types, field_trait.plan);
        let visibility = FieldTraitPath::new(self.program).visibility(field_trait.binding);
        vec![
            Fragment::Text(format!("{visibility}trait {} {{", field_trait.path)),
            Fragment::Indented(vec![Fragment::Text(format!(
                "fn {}(&self) -> {read};",
                field_trait.getter
            ))]),
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

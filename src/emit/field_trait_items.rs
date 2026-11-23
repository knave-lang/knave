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
    const REFERENCES: [&'static str; 2] = ["&", "&mut "];

    pub fn new(program: &'a TypedProgram, plan: &'a EmitPlan<'a>, types: EmittedTypes<'a>) -> Self {
        Self {
            program,
            plan,
            types,
        }
    }

    pub fn of_module(&self, module: ModuleId) -> Vec<Vec<Fragment>> {
        let field_traits = self
            .program
            .declarations
            .traits()
            .rust_paths()
            .field_traits();
        if field_traits.plans().next().is_none() {
            return Vec::new();
        }
        let traits: Vec<FieldTrait> = (field_traits.plans())
            .filter_map(|(binding, plan)| self.field_trait(binding, plan, module))
            .collect();
        if traits.is_empty() {
            return Vec::new();
        }
        let trait_module = (module == self.program.entry).then(|| self.trait_module(&traits));
        trait_module
            .into_iter()
            .chain(self.implementations_in(module, &traits))
            .collect()
    }

    fn implementations_in(&self, module: ModuleId, traits: &[FieldTrait]) -> Vec<Vec<Fragment>> {
        let types = RustTypes::new(self.program, self.plan, self.types, module);
        let mut implementations = Vec::new();
        for field_trait in traits {
            let local = (field_trait.plan.implementors.iter())
                .filter(|implementor| self.emitted_module(implementor) == Some(module));
            implementations.extend(
                local.map(|implementor| self.implementation(&types, field_trait, implementor)),
            );
        }
        implementations
    }

    fn emitted_module(&self, implementor: &Type) -> Option<ModuleId> {
        let Type::Named(binding, _) = implementor else {
            return None;
        };
        self.plan.item(*binding).map(|item| item.module)
    }

    fn trait_module(&self, traits: &[FieldTrait]) -> Vec<Fragment> {
        let rooted = FieldTraitPath::outside_every_module();
        let types = RustTypes::new(self.program, self.plan, self.types, rooted);
        let mut inner = vec![Fragment::Newline];
        let declarations = traits.iter().flat_map(|field_trait| {
            let declaration = self.declaration(&types, field_trait);
            let references = (Self::REFERENCES.iter())
                .map(|reference| Self::reference_impl(&types, field_trait, reference));
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

    fn field_trait<'p>(
        &self,
        binding: BindingId,
        plan: &'p FieldTraitPlan,
        module: ModuleId,
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
            path: FieldTraitPath::new(self.program).of(binding, module)?,
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

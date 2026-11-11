use super::declarations::Declarations;
use super::declared_traits::DeclaredTraits;
use super::derive_blockers::DeriveBlockers;
use super::trait_declarations::TraitDeclarations;
use super::type_declarations::TypeDeclarations;
use super::variant_payloads::VariantPayloads;
use super::{TypedProgram, TypedSource};
use crate::core::BindingId;

pub(super) struct DeclarationsFixture {
    program: TypedProgram,
    declarations: Declarations,
}

impl DeclarationsFixture {
    pub(super) fn new(source: &str) -> Self {
        let mut program = TypedSource::expand(source).typed();
        let types = TypeDeclarations {
            structs: std::mem::take(&mut program.structs),
            enums: std::mem::take(&mut program.enums),
            rust_types: std::mem::take(&mut program.rust_types),
            box_binding: program.box_binding,
            inferred_boxes: std::mem::take(&mut program.inferred_boxes),
        };
        let traits = TraitDeclarations {
            table: std::mem::take(&mut program.traits),
            impls: std::mem::take(&mut program.trait_impls),
            structural: std::mem::take(&mut program.structural_traits),
            ..TraitDeclarations::default()
        };
        Self {
            program,
            declarations: Declarations { types, traits },
        }
    }

    pub(super) fn declared_traits(&self) -> DeclaredTraits<'_> {
        DeclaredTraits::new(&self.declarations.traits)
    }

    pub(super) fn derive_blockers(&self) -> DeriveBlockers<'_> {
        DeriveBlockers::new(
            &self.declarations.types,
            self.declared_traits(),
            VariantPayloads::new(&self.declarations.types),
        )
    }

    pub(super) fn trait_named(&self, name: &str) -> BindingId {
        self.binding_named(name, &|binding| {
            self.declarations.traits.table.get(binding).is_some()
        })
    }

    pub(super) fn type_named(&self, name: &str) -> BindingId {
        let types = &self.declarations.types;
        self.binding_named(name, &|binding| {
            types.structs.get(binding).is_some() || types.enums.get(binding).is_some()
        })
    }

    fn binding_named(&self, name: &str, declared: &dyn Fn(BindingId) -> bool) -> BindingId {
        self.program
            .bindings
            .ids()
            .find(|binding| {
                self.program.bindings.get(*binding).name() == name && declared(*binding)
            })
            .expect("the source declares the name")
    }
}

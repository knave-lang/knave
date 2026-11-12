use super::declarations::Declarations;
use super::declared_traits::DeclaredTraits;
use super::declared_types::DeclaredTypes;
use super::derive_blockers::DeriveBlockers;
use super::trait_boxing::TraitBoxing;
use super::type_declarations::TypeDeclarations;
use super::variant_payloads::VariantPayloads;
use super::{Type, TypedProgram, TypedSource};
use crate::core::BindingId;

pub(super) struct DeclarationsFixture {
    program: TypedProgram,
    declarations: Declarations,
}

impl DeclarationsFixture {
    pub(super) fn new(source: &str) -> Self {
        let mut program = TypedSource::expand(source).typed();
        let declarations = std::mem::take(&mut program.declarations);
        Self {
            program,
            declarations,
        }
    }

    pub(super) fn types(&self) -> &TypeDeclarations {
        &self.declarations.types
    }

    pub(super) fn trait_boxing(&self) -> TraitBoxing<'_> {
        TraitBoxing::new(&self.declarations.traits, &self.program.bindings)
    }

    pub(super) fn declared_types(&self) -> DeclaredTypes<'_> {
        DeclaredTypes::new(&self.declarations.types)
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

    pub(super) fn parameter(&self, function: &str) -> Type {
        let definition = self
            .program
            .modules
            .iter()
            .flat_map(|module| &module.definitions)
            .find(|definition| self.program.bindings.get(definition.binding).name() == function)
            .expect("the source defines the function");
        let Type::Function(_, parameters, _) = &definition.scheme.body else {
            panic!("a defn has a function type");
        };
        parameters[0].clone()
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

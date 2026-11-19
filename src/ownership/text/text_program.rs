use rustc_hash::FxHashMap;

use super::text_names::TextNames;
use crate::core::{BindingId, BindingKind, ClosureKind, PrimitiveType};
use crate::modules::ModuleSources;
use crate::types::{
    Bound, GenericId, StructDefinition, Type, TypedDefinition, TypedModule, TypedProgram,
    VariantPayload,
};

pub struct TextProgram<'p> {
    program: &'p TypedProgram,
    definitions: FxHashMap<BindingId, &'p TypedDefinition>,
}

impl<'p> TextProgram<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        let definitions = program
            .modules
            .iter()
            .flat_map(|module| &module.definitions)
            .map(|definition| (definition.binding, definition))
            .collect();
        Self {
            program,
            definitions,
        }
    }

    pub fn written_modules(&self) -> impl Iterator<Item = &'p TypedModule> + '_ {
        self.program
            .modules
            .iter()
            .filter(|module| !ModuleSources::is_prelude(&module.path))
    }

    pub fn definition(&self, binding: BindingId) -> Option<&'p TypedDefinition> {
        self.definitions.get(&binding).copied()
    }

    pub fn structure_of(&self, ty: &Type) -> Option<(BindingId, &'p StructDefinition)> {
        match ty {
            Type::Ref(_, inner) | Type::Mut(_, inner) => self.structure_of(inner),
            Type::Named(binding, _) => self
                .program
                .declarations
                .types()
                .structs()
                .get(*binding)
                .map(|structure| (*binding, structure)),
            _ => None,
        }
    }

    pub fn payload_argument(&self, variant: BindingId, field: usize) -> Option<usize> {
        let BindingKind::Variant(_, info) = self.program.bindings.get(variant).kind() else {
            return None;
        };
        let definition = self.program.declarations.types().enums().get(info.owner)?;
        let payload = match &definition.variants.get(info.index)?.payload {
            VariantPayload::Positional(types) => types.get(field)?,
            VariantPayload::Named(fields) => &fields.get(field)?.1,
            VariantPayload::None => return None,
        };
        match payload {
            Type::Generic(generic) => definition
                .params
                .iter()
                .position(|param| param.id == *generic),
            _ => None,
        }
    }

    pub fn owns_text_for(&self, bound: &Bound) -> bool {
        let table = self.program.declarations.traits().table();
        let anything = |_: GenericId, _: &Bound| true;
        let holds = |ty: Type| table.satisfies(bound, &ty, &anything);
        let holds_for_every_reference = holds(Type::shared(Self::function_type()));
        let views_hold = holds(Type::Primitive(PrimitiveType::Str))
            || holds(Type::str_ref()) && !holds_for_every_reference;
        holds(Type::Primitive(PrimitiveType::String)) && !views_hold
    }

    pub fn names(&self) -> TextNames<'p> {
        TextNames::new(self.program)
    }

    fn function_type() -> Type {
        Type::Function(ClosureKind::Fn, Vec::new(), Box::new(Type::unit()))
    }
}

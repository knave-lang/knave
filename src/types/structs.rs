use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::ty::{GenericId, GenericParam, Type};
use crate::core::BindingId;
use crate::diagnostic::SimilarName;
use crate::source::Span;

#[derive(Clone)]
pub struct StructDefinition {
    pub lifetimes: Vec<BindingId>,
    pub params: Vec<GenericParam>,
    pub fields: Vec<(Rc<str>, Type)>,
    pub field_spans: Vec<Span>,
    pub bare_text_fields: Vec<Rc<str>>,
    pub function_spans: Vec<Span>,
    pub derives: Vec<&'static str>,
    pub derived_traits: Vec<BindingId>,
    pub blocked: Vec<(BindingId, Rc<str>, Type)>,
    pub written_derive: Option<Span>,
}

impl StructDefinition {
    pub fn closest_field(&self, name: &str) -> Option<Rc<str>> {
        SimilarName::closest(name, &self.field_names().collect::<Vec<_>>())
    }

    fn field_names(&self) -> impl Iterator<Item = Rc<str>> + '_ {
        self.fields.iter().map(|(name, _)| name.clone())
    }

    pub fn field(&self, name: &str) -> Option<&Type> {
        self.fields
            .iter()
            .find(|(field, _)| field.as_ref() == name)
            .map(|(_, field_type)| field_type)
    }

    pub fn instantiated_field(&self, name: &str, arguments: &[Type]) -> Option<Type> {
        let replacements = self.replacements(arguments);
        self.field(name).map(|field_type| {
            field_type
                .without_written_lifetimes()
                .substitute(&replacements)
        })
    }

    pub fn holds_reference_in(&self, field: &str, arguments: &[Type]) -> bool {
        self.bare_text_fields
            .iter()
            .any(|bare| bare.as_ref() == field)
            || self
                .instantiated_field(field, arguments)
                .is_some_and(|field_type| field_type.is_reference())
    }

    pub fn instantiated_fields(&self, arguments: &[Type]) -> Vec<(Rc<str>, Type)> {
        let replacements = self.replacements(arguments);
        self.fields
            .iter()
            .map(|(name, field_type)| {
                (
                    name.clone(),
                    field_type
                        .without_written_lifetimes()
                        .substitute(&replacements),
                )
            })
            .collect()
    }

    fn replacements(&self, arguments: &[Type]) -> FxHashMap<GenericId, Type> {
        self.params
            .iter()
            .map(|param| param.id)
            .zip(arguments.iter().cloned())
            .collect()
    }
}

pub struct StructBehind<'t, 'a> {
    pub binding: BindingId,
    pub definition: &'t StructDefinition,
    pub arguments: &'a [Type],
}

#[derive(Clone, Default)]
pub struct StructTable {
    structs: FxHashMap<BindingId, StructDefinition>,
}

impl StructTable {
    pub fn add(&mut self, binding: BindingId, definition: StructDefinition) {
        self.structs.insert(binding, definition);
    }

    pub fn behind<'a>(&self, ty: &'a Type) -> Option<StructBehind<'_, 'a>> {
        match ty {
            Type::Ref(_, inner) | Type::Mut(_, inner) => self.behind(inner),
            Type::Named(binding, arguments) => self.get(*binding).map(|definition| StructBehind {
                binding: *binding,
                definition,
                arguments,
            }),
            _ => None,
        }
    }

    pub fn get(&self, binding: BindingId) -> Option<&StructDefinition> {
        self.structs.get(&binding)
    }

    pub fn get_mut(&mut self, binding: BindingId) -> Option<&mut StructDefinition> {
        self.structs.get_mut(&binding)
    }

    pub fn iter(&self) -> impl Iterator<Item = (BindingId, &StructDefinition)> {
        self.structs
            .iter()
            .map(|(binding, definition)| (*binding, definition))
    }

    pub fn closest_field(&self, name: &str) -> Option<Rc<str>> {
        let names: Vec<Rc<str>> = self
            .structs
            .values()
            .flat_map(StructDefinition::field_names)
            .collect();
        SimilarName::closest(name, &names)
    }

    pub fn with_field(&self, name: &str) -> Vec<BindingId> {
        self.structs
            .iter()
            .filter(|(_, definition)| definition.field(name).is_some())
            .map(|(binding, _)| *binding)
            .collect()
    }
}

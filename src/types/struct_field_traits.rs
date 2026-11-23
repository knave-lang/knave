use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::copy::CopyRules;
use super::declarations::Declarations;
use super::field_trait::FieldTrait;
use super::implementor::Implementor;
use super::open_read::OpenRead;
use super::structs::{StructDefinition, StructTable};
use super::ty::{GenericId, GenericParam, Scheme, Type};
use crate::core::BindingId;

pub(super) struct StructFieldTraits<'a> {
    structs: &'a StructTable,
    reads: OpenRead<'a>,
}

impl<'a> StructFieldTraits<'a> {
    pub(super) fn new(
        declarations: &'a Declarations,
        generics: &'a FxHashMap<GenericId, GenericParam>,
    ) -> Self {
        Self {
            structs: &declarations.types().structs,
            reads: OpenRead::new(CopyRules::new(declarations, generics)),
        }
    }

    pub(super) fn traits(
        &self,
        written: &FxHashSet<Rc<str>>,
        read: &FxHashSet<Rc<str>>,
    ) -> Vec<FieldTrait> {
        let mut shared: Vec<&Rc<str>> = (written.iter().chain(read))
            .filter(|field| self.is_shared(field))
            .collect();
        shared.sort();
        shared.dedup();
        shared
            .into_iter()
            .map(|field| FieldTrait::new(Rc::clone(field), self.implementors(field)))
            .collect()
    }

    fn is_shared(&self, field: &str) -> bool {
        let mut declaring =
            (self.structs.iter()).filter(|(_, definition)| definition.field(field).is_some());
        declaring.nth(1).is_some()
    }

    fn implementors(&self, field: &str) -> Vec<(BindingId, Implementor)> {
        let mut found: Vec<(BindingId, Implementor)> = self
            .structs
            .iter()
            .filter_map(|(binding, definition)| {
                let read = self.reads.of(definition, field)?.read;
                Some((binding, Self::implementor(binding, definition, read)))
            })
            .collect();
        found.sort_by_key(|(binding, _)| *binding);
        found
    }

    fn implementor(binding: BindingId, definition: &StructDefinition, read: Type) -> Implementor {
        let arguments = definition
            .params
            .iter()
            .map(|param| Type::Generic(param.id))
            .collect();
        Implementor {
            scheme: Scheme {
                lifetimes: Vec::new(),
                params: definition.params.clone(),
                body: Type::Named(binding, arguments),
            },
            argument: Some(read),
        }
    }
}

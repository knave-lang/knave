use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::copy::CopyRules;
use super::declarations::Declarations;
use super::field_traits::FieldTrait;
use super::open_read::OpenRead;
use super::structs::{StructDefinition, StructTable};
use super::trait_rows_reach::Implementor;
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

    pub(super) fn traits(&self) -> Vec<FieldTrait> {
        self.shared_names()
            .into_iter()
            .map(|field| FieldTrait {
                implementors: self.implementors(&field),
                field,
            })
            .collect()
    }

    fn shared_names(&self) -> Vec<Rc<str>> {
        let mut names: Vec<Rc<str>> = self
            .structs
            .iter()
            .flat_map(|(_, definition)| definition.fields.iter().map(|(name, _)| name.clone()))
            .collect();
        names.sort();
        let mut shared: Vec<Rc<str>> = names
            .chunk_by(|left, right| left == right)
            .map(|group| group[0].clone())
            .collect();
        shared.dedup();
        shared
    }

    fn implementors(&self, field: &str) -> Vec<(BindingId, Implementor)> {
        let mut found: Vec<(BindingId, Implementor)> = self
            .structs
            .iter()
            .filter_map(|(binding, definition)| {
                let (read, _) = self.reads.of(definition, field)?;
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

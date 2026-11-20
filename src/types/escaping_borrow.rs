use rustc_hash::FxHashMap;

use super::field_reader::FieldReader;
use super::field_traits::FieldTraitPlan;
use super::read_kind::ReadKind;
use super::ty::{GenericId, Type};
use super::typed::TypedProgram;
use super::unborrowed::Unborrowed;

pub(super) struct EscapingBorrow<'p> {
    program: &'p TypedProgram,
}

impl<'p> EscapingBorrow<'p> {
    pub(super) fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub(super) fn escapes(&self, reader: &FieldReader, plan: &FieldTraitPlan) -> bool {
        plan.kind != ReadKind::BorrowOfStruct && self.escapes_copied_owner(reader, plan)
    }

    pub(super) fn escapes_copied_owner(&self, reader: &FieldReader, plan: &FieldTraitPlan) -> bool {
        if !self.holds_borrow(&plan.read) {
            return false;
        }
        let settled: FxHashMap<GenericId, Type> = match reader.read {
            Type::Generic(open) => std::iter::once((open, plan.read.clone())).collect(),
            _ => FxHashMap::default(),
        };
        let exits = self.exits(reader).into_iter();
        exits
            .map(|exit| exit.substitute(&settled))
            .any(|exit| self.holds_borrow(&exit))
    }

    fn exits(&self, reader: &FieldReader) -> Vec<Type> {
        let definitions = (self.program.modules.iter()).flat_map(|module| &module.definitions);
        let mut readers = definitions.filter(|definition| definition.binding == reader.binding);
        match readers.next().map(|definition| &definition.scheme.body) {
            Some(Type::Function(_, parameters, result)) => {
                let holders = parameters
                    .iter()
                    .map(|parameter| Unborrowed::of(parameter).clone());
                holders
                    .chain(std::iter::once(result.as_ref().clone()))
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn holds_borrow(&self, read: &Type) -> bool {
        match read {
            Type::Ref(..) | Type::Mut(..) | Type::Function(..) => true,
            Type::Named(binding, arguments) => {
                self.program.has_lifetime_parameter(*binding)
                    || arguments.iter().any(|argument| self.holds_borrow(argument))
            }
            unsized_view => unsized_view.is_unsized(),
        }
    }
}

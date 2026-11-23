use rustc_hash::FxHashMap;

use super::copy::CopyRules;
use super::field_read::FieldRead;
use super::read_kind::ReadKind;
use super::structs::StructDefinition;
use super::ty::{GenericId, Type};

pub(super) struct OpenRead<'a> {
    copy: CopyRules<'a>,
}

impl<'a> OpenRead<'a> {
    pub(super) fn new(copy: CopyRules<'a>) -> Self {
        Self { copy }
    }

    pub(super) fn of(&self, definition: &StructDefinition, field: &str) -> Option<FieldRead> {
        let declared = definition.field(field)?;
        Some(self.classified(declared, declared.without_written_lifetimes()))
    }

    pub(super) fn instantiated(
        &self,
        definition: &StructDefinition,
        field: &str,
        arguments: &[Type],
    ) -> Option<FieldRead> {
        let declared = definition.field(field)?;
        let field_type = definition.instantiated_field(field, arguments)?;
        Some(self.classified(declared, field_type))
    }

    pub(super) fn through_bound(
        &self,
        definition: &StructDefinition,
        field: &str,
        arguments: &[Type],
    ) -> Option<FieldRead> {
        let declared = self.of(definition, field)?;
        let replacements: FxHashMap<GenericId, Type> =
            (definition.params.iter().map(|param| param.id))
                .zip(arguments.iter().cloned())
                .collect();
        Some(FieldRead {
            read: declared.read.substitute(&replacements),
            kind: declared.kind,
        })
    }

    pub(super) fn of_written(&self, written: &Type) -> Type {
        if !written.is_concrete() {
            return written.clone();
        }
        self.classified(written, written.clone()).read
    }

    fn classified(&self, declared: &Type, field_type: Type) -> FieldRead {
        let kind = match declared {
            Type::Ref(..) => ReadKind::StoredBorrow,
            Type::Function(..) => ReadKind::Value,
            _ if self.copy.is_small_copy(&field_type) => ReadKind::Value,
            _ => ReadKind::BorrowOfStruct,
        };
        let read = match kind {
            ReadKind::BorrowOfStruct => Type::shared(field_type),
            ReadKind::Value | ReadKind::StoredBorrow => field_type,
        };
        FieldRead { read, kind }
    }
}

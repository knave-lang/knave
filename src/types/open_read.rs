use super::copy::CopyRules;
use super::read_kind::ReadKind;
use super::structs::StructDefinition;
use super::ty::Type;

pub(super) struct OpenRead<'a> {
    copy: CopyRules<'a>,
}

impl<'a> OpenRead<'a> {
    pub(super) fn new(copy: CopyRules<'a>) -> Self {
        Self { copy }
    }

    pub(super) fn of(
        &self,
        definition: &StructDefinition,
        field: &str,
    ) -> Option<(Type, ReadKind)> {
        let declared = definition.field(field)?.without_written_lifetimes();
        Some(match declared {
            Type::Ref(..) => (declared, ReadKind::StoredBorrow),
            Type::Function(..) => (declared, ReadKind::Value),
            _ if self.copy.is_small_copy(&declared) => (declared, ReadKind::Value),
            _ => (Type::shared(declared), ReadKind::BorrowOfStruct),
        })
    }
}

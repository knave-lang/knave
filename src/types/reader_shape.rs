use super::copy::CopyRules;
use super::escaping_borrow::EscapingBorrow;
use super::field_changes::FieldChanges;
use super::field_reader::FieldReader;
use super::field_trait_names::FieldTraitNames;
use super::field_traits::FieldTraitPlan;
use super::owner_uses::OwnerUses;
use super::ty::Type;
use super::typed::{TypedDefinition, TypedProgram};
use super::written_refusal::{ReadRefusal, UseRefusal, WrittenRefusal};

pub(super) struct ReaderShape<'p> {
    program: &'p TypedProgram,
    copy: CopyRules<'p>,
}

impl<'p> ReaderShape<'p> {
    pub(super) fn new(program: &'p TypedProgram) -> Self {
        Self {
            program,
            copy: CopyRules::new(&program.declarations, &program.generics),
        }
    }

    pub(super) fn refusal(
        &self,
        reader: &FieldReader,
        plan: &FieldTraitPlan,
    ) -> Option<WrittenRefusal> {
        let names = FieldTraitNames::new(self.program);
        if matches!(plan.read, Type::Function(..)) {
            Some(WrittenRefusal::Read(ReadRefusal::Function))
        } else if self.changes_a_field(reader) {
            Some(WrittenRefusal::Use(UseRefusal::ChangesField))
        } else if !self.passes_owner(reader, plan) {
            Some(WrittenRefusal::Use(UseRefusal::MovesOwner))
        } else if self.escapes(reader, plan) {
            Some(WrittenRefusal::Use(UseRefusal::BorrowLeaves))
        } else if !names.are_free(reader)
            || (self.copies_owner(plan) && names.copy_takes_getter(reader))
        {
            Some(WrittenRefusal::Use(UseRefusal::NameTaken))
        } else {
            None
        }
    }

    pub(super) fn copies_owner(&self, plan: &FieldTraitPlan) -> bool {
        (plan.implementors.iter()).all(|implementor| self.copy.is_copy(implementor))
    }

    fn escapes(&self, reader: &FieldReader, plan: &FieldTraitPlan) -> bool {
        let escaping = EscapingBorrow::new(self.program);
        if self.copies_owner(plan) {
            escaping.escapes_copied_owner(reader, plan)
        } else {
            escaping.escapes(reader, plan)
        }
    }

    fn passes_owner(&self, reader: &FieldReader, plan: &FieldTraitPlan) -> bool {
        self.copies_owner(plan) || !self.uses_owner_by_value(reader)
    }

    fn uses_owner_by_value(&self, reader: &FieldReader) -> bool {
        let definition = self.definition(reader);
        definition.is_some_and(|definition| OwnerUses::new(reader).take_owner_by_value(definition))
    }

    fn changes_a_field(&self, reader: &FieldReader) -> bool {
        let definition = self.definition(reader);
        definition.is_some_and(|definition| FieldChanges::new(reader).within(&definition.value))
    }

    fn definition(&self, reader: &FieldReader) -> Option<&'p TypedDefinition> {
        let mut definitions = (self.program.modules.iter()).flat_map(|module| &module.definitions);
        definitions.find(|definition| definition.binding == reader.binding)
    }
}

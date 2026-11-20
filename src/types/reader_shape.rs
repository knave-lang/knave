use super::copy::CopyRules;
use super::escaping_borrow::EscapingBorrow;
use super::field_changes::FieldChanges;
use super::field_reader::FieldReader;
use super::field_trait_names::FieldTraitNames;
use super::field_traits::FieldTraitPlan;
use super::mentioned_generics::MentionedGenerics;
use super::owner_uses::OwnerUses;
use super::ty::{Bound, GenericId, Type};
use super::typed::{TypedDefinition, TypedProgram};

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

    pub(super) fn emits(&self, reader: &FieldReader, plan: &FieldTraitPlan) -> bool {
        let names = FieldTraitNames::new(self.program);
        self.allows(reader, plan)
            && !self.escapes(reader, plan)
            && names.are_free(reader)
            && !(self.copies_owner(plan) && names.copy_takes_getter(reader))
    }

    fn escapes(&self, reader: &FieldReader, plan: &FieldTraitPlan) -> bool {
        let escaping = EscapingBorrow::new(self.program);
        if self.copies_owner(plan) {
            escaping.escapes_copied_owner(reader, plan)
        } else {
            escaping.escapes(reader, plan)
        }
    }

    fn allows(&self, reader: &FieldReader, plan: &FieldTraitPlan) -> bool {
        let emitted_read = plan.read.is_concrete() && !matches!(plan.read, Type::Function(..));
        let settled_read = matches!(reader.read, Type::Generic(_)) || reader.read.is_concrete();
        emitted_read
            && settled_read
            && !self.bounds_the_read(reader, plan)
            && !self.changes_a_field(reader)
            && self.passes_owner(reader, plan)
    }

    fn bounds_the_read(&self, reader: &FieldReader, plan: &FieldTraitPlan) -> bool {
        let Some(definition) = self.definition(reader) else {
            return false;
        };
        let field_traits = self
            .program
            .declarations
            .traits()
            .rust_paths()
            .field_traits();
        let is_field_bound =
            |bound: &Bound| field_traits.bindings().field_of(bound.binding).is_some();
        let params = &definition.scheme.params;
        let field_bounds = (params.iter())
            .flat_map(|param| &param.bounds)
            .filter(|bound| is_field_bound(bound));
        let reads: Vec<GenericId> = (field_bounds.filter_map(|bound| bound.argument.as_ref()))
            .flat_map(|read| MentionedGenerics::of(read).into_ids())
            .collect();
        let mut others = (params.iter())
            .flat_map(|param| param.bounds.iter().map(move |bound| (param.id, bound)))
            .filter(|(_, bound)| !is_field_bound(bound));
        others.any(|(bounded, bound)| {
            let named = bound
                .argument
                .as_ref()
                .map(|argument| MentionedGenerics::of(argument).into_ids());
            let named = named.unwrap_or_default();
            let touches = reads.contains(&bounded) || named.iter().any(|id| reads.contains(id));
            let ties_other = named.iter().any(|id| !reads.contains(id));
            let on_own_read = reader.read == Type::Generic(bounded) && named.is_empty();
            touches && (ties_other || (on_own_read && !self.satisfied(bound, &plan.read)))
        })
    }

    fn satisfied(&self, bound: &Bound, read: &Type) -> bool {
        let traits = self.program.declarations.traits().table();
        traits.satisfies(bound, read, &|_, _| false)
    }

    pub(super) fn copies_owner(&self, plan: &FieldTraitPlan) -> bool {
        (plan.implementors.iter()).all(|implementor| self.copy.is_copy(implementor))
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

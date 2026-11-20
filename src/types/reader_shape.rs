use super::copy::CopyRules;
use super::escaping_borrow::EscapingBorrow;
use super::field_reader::FieldReader;
use super::field_trait_names::FieldTraitNames;
use super::field_traits::FieldTraitPlan;
use super::ty::Type;
use super::typed::{
    Borrow, TypedArgument, TypedDefinition, TypedExpr, TypedExprKind, TypedLeaf, TypedProgram,
    TypedStructExpr,
};
use super::unborrowed::Unborrowed;
use crate::core::{BindingId, ReferenceKind};

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
            && !EscapingBorrow::new(self.program).escapes(reader, plan)
            && names.are_free(reader)
            && !(self.copies_owner(plan) && names.copy_takes_getter(reader))
    }

    fn allows(&self, reader: &FieldReader, plan: &FieldTraitPlan) -> bool {
        let emitted_read = plan.read.is_concrete() && !matches!(plan.read, Type::Function(..));
        emitted_read && !self.changes_a_field(reader) && self.passes_owner(reader, plan)
    }

    pub(super) fn copies_owner(&self, plan: &FieldTraitPlan) -> bool {
        (plan.implementors.iter()).all(|implementor| self.copy.is_copy(implementor))
    }

    fn passes_owner(&self, reader: &FieldReader, plan: &FieldTraitPlan) -> bool {
        self.copies_owner(plan) || !self.uses_owner_by_value(reader)
    }

    fn uses_owner_by_value(&self, reader: &FieldReader) -> bool {
        let Some(TypedExprKind::Fn(lambda)) =
            self.definition(reader).map(|found| &found.value.kind)
        else {
            return false;
        };
        let owner = Type::Generic(reader.owner);
        let mut owners = (lambda.params.iter()).filter(|param| Unborrowed::of(&param.ty) == &owner);
        let body = lambda.body.items.iter().map(|item| item.expr());
        owners.any(|param| {
            body.clone()
                .any(|expr| Self::uses_by_value(expr, param.binding))
        })
    }

    fn uses_by_value(expr: &TypedExpr, owner: BindingId) -> bool {
        match &expr.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => *binding == owner,
            TypedExprKind::Struct(TypedStructExpr::Field(value, _))
                if Self::is_owner(value, owner) =>
            {
                false
            }
            _ => (expr.children().into_iter()).any(|child| Self::uses_by_value(child, owner)),
        }
    }

    fn is_owner(value: &TypedExpr, owner: BindingId) -> bool {
        matches!(value.kind, TypedExprKind::Leaf(TypedLeaf::Local(local)) if local == owner)
    }

    fn changes_a_field(&self, reader: &FieldReader) -> bool {
        let definition = self.definition(reader);
        definition.is_some_and(|definition| Self::changes_in(&definition.value, reader))
    }

    fn definition(&self, reader: &FieldReader) -> Option<&'p TypedDefinition> {
        let mut definitions = (self.program.modules.iter()).flat_map(|module| &module.definitions);
        definitions.find(|definition| definition.binding == reader.binding)
    }

    fn changes_in(expr: &TypedExpr, reader: &FieldReader) -> bool {
        Self::changes_here(expr, reader)
            || expr
                .children()
                .into_iter()
                .any(|child| Self::changes_in(child, reader))
    }

    fn changes_here(expr: &TypedExpr, reader: &FieldReader) -> bool {
        match &expr.kind {
            TypedExprKind::Borrow(ReferenceKind::Mutable, place) => {
                Self::reads_owner(place, reader)
            }
            TypedExprKind::Call(callee, arguments) => {
                Self::passes_changed_field(callee, arguments, reader)
            }
            _ => false,
        }
    }

    fn passes_changed_field(
        callee: &TypedExpr,
        arguments: &[TypedArgument],
        reader: &FieldReader,
    ) -> bool {
        let parameters = match &callee.ty {
            Type::Function(_, parameters, _) => parameters.as_slice(),
            _ => &[],
        };
        let mut passed = arguments.iter().enumerate();
        passed.any(|(position, argument)| {
            let changing = argument.borrow == Borrow::Mutable
                || matches!(parameters.get(position), Some(Type::Mut(..)));
            changing && Self::reads_owner(&argument.value, reader)
        })
    }

    fn reads_owner(place: &TypedExpr, reader: &FieldReader) -> bool {
        match &place.kind {
            TypedExprKind::Struct(TypedStructExpr::Field(value, _)) => {
                Unborrowed::of(&value.ty) == &Type::Generic(reader.owner)
                    || Self::reads_owner(value, reader)
            }
            _ => false,
        }
    }
}

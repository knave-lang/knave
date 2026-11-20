use super::copy::CopyRules;
use super::declarations::Declarations;
use super::field_owner_problems::FieldOwnerProblems;
use super::open_field_owner::OpenFieldOwner;
use super::open_read::OpenRead;
use super::plain_bound::PlainBound;
use super::read_view::ReadView;
use super::ty::Type;
use super::typed::{TypedExpr, TypedExprKind, TypedStructExpr};
use super::unifier::Unifier;
use crate::core::{BindingId, FieldLabel};
use crate::diagnostic::Diagnostic;

pub(super) struct UnfitReads<'c> {
    unifier: &'c Unifier,
    declarations: &'c Declarations,
    reads: OpenRead<'c>,
}

impl<'c> UnfitReads<'c> {
    pub(super) fn new(
        unifier: &'c Unifier,
        declarations: &'c Declarations,
        copy: CopyRules<'c>,
    ) -> Self {
        Self {
            unifier,
            declarations,
            reads: OpenRead::new(copy),
        }
    }

    pub(super) fn refuse(&self, value: &TypedExpr) -> Result<(), Diagnostic> {
        let field_traits = self.declarations.traits().rust_paths().field_traits();
        if field_traits.bindings().is_empty() {
            return Ok(());
        }
        match self.first_unfit(value) {
            Some(label) => Err(FieldOwnerProblems::ambiguous(&label.name, label.span)),
            None => Ok(()),
        }
    }

    fn first_unfit<'e>(&self, expr: &'e TypedExpr) -> Option<&'e FieldLabel> {
        if let TypedExprKind::Struct(TypedStructExpr::Field(value, label)) = &expr.kind
            && self.is_unfit(&value.ty, &label.name)
        {
            return Some(label);
        }
        (expr.children().into_iter()).find_map(|child| self.first_unfit(child))
    }

    fn is_unfit(&self, owner: &Type, field: &str) -> bool {
        let Some(read) = self.open_read(owner, field) else {
            return false;
        };
        let structs = self.declarations.types().structs.with_field(field);
        !structs
            .into_iter()
            .any(|owner| self.gives(owner, field, &read))
    }

    fn open_read(&self, owner: &Type, field: &str) -> Option<Type> {
        let field_traits = self.declarations.traits().rust_paths().field_traits();
        let open = OpenFieldOwner::new(self.unifier);
        let read = open.bound_read(open.var(owner)?, field_traits.bindings().trait_of(field)?)?;
        Some(self.unifier.resolve(&read))
    }

    fn gives(&self, owner: BindingId, field: &str, used: &Type) -> bool {
        let definition = self.declarations.types().structs.get(owner);
        let read = definition.and_then(|definition| self.reads.of(definition, field));
        read.is_some_and(|read| {
            ReadView::of(&read.read).is_used_as(used) && self.meets_bounds(&read.read, used)
        })
    }

    fn meets_bounds(&self, read: &Type, used: &Type) -> bool {
        let Type::Var(var) = used else {
            return true;
        };
        let traits = self.declarations.traits();
        let field_traits = traits.rust_paths().field_traits();
        let mut plain = (self.unifier.bounds(*var).iter()).filter(|bound| {
            bound.argument.is_none() && !field_traits.bindings().is_field_bound(bound)
        });
        plain.all(|bound| PlainBound::new(traits.table()).is_met_by(bound, read))
    }
}

use super::bound_locals::{BoundLocal, BoundLocals};
use super::built_parts::BuiltParts;
use super::call_result_sources::CallResultSources;
use super::kept_temporary_problems::KeptValue;
use super::promoted_constants::PromotedConstants;
use super::result_source::ResultSource;
use crate::types::{
    PositionTie, ReturnedValues, Type, TypedBinder, TypedExpr, TypedExprKind, TypedProgram,
};

pub struct LentTemporary<'p> {
    program: &'p TypedProgram,
    sources: CallResultSources<'p>,
    parts: BuiltParts<'p>,
    constants: PromotedConstants<'p>,
}

impl<'p> LentTemporary<'p> {
    pub fn new(
        program: &'p TypedProgram,
        sources: CallResultSources<'p>,
        parts: BuiltParts<'p>,
    ) -> Self {
        Self {
            program,
            sources,
            parts,
            constants: PromotedConstants::new(program),
        }
    }

    pub fn lent_in<'e>(
        &self,
        parameter: &TypedBinder,
        argument: &'e TypedExpr,
    ) -> Option<LentValue<'e>> {
        let walk = ArgumentWalk {
            lent: self,
            bound: BoundLocals::in_argument(self.program, argument, &self.parts),
        };
        let value =
            walk.first_temporary([ResultSource::kept_by_closure(&parameter.ty, argument)])?;
        let kind = if walk.bound.owns(value) {
            KeptValue::BlockLocal
        } else {
            KeptValue::Temporary
        };
        Some(LentValue { value, kind })
    }
}

pub struct LentValue<'e> {
    pub value: &'e TypedExpr,
    pub kind: KeptValue,
}

struct ArgumentWalk<'w, 'p, 'e> {
    lent: &'w LentTemporary<'p>,
    bound: BoundLocals<'w, 'p, 'e>,
}

type Follow<'w, 'p, 'e> = fn(&ArgumentWalk<'w, 'p, 'e>, &'e TypedExpr) -> Option<&'e TypedExpr>;

impl<'w, 'p, 'e> ArgumentWalk<'w, 'p, 'e> {
    fn lent_by(&self, argument: &'e TypedExpr) -> Option<&'e TypedExpr> {
        let value = match &argument.kind {
            TypedExprKind::Borrow(_, value) => value,
            _ => argument,
        };
        if let Some(found) = self.through_bound(value, Self::lent_by) {
            return found;
        }
        if value.ty.is_reference() {
            return self.borrowed_through(value);
        }
        let owns_its_value = !value.is_place() && !self.lent.constants.is_promoted(value);
        owns_its_value.then_some(value)
    }

    fn held_in(&self, value: &'e TypedExpr) -> Option<&'e TypedExpr> {
        if value.ty.is_reference() {
            return self.borrowed_through(value);
        }
        if !self.lent.program.type_holds_borrow(&value.ty) {
            return None;
        }
        if let Some(found) = self.through_bound(value, Self::held_in) {
            return found;
        }
        if let Some(sources) = self.lent.sources.of(value) {
            return self.first_temporary(sources);
        }
        if let Some(chosen) = self.chosen_values(value) {
            return chosen.into_iter().find_map(|value| self.held_in(value));
        }
        value
            .children()
            .into_iter()
            .find_map(|child| self.held_in(child))
    }

    fn borrowed_through(&self, reference: &'e TypedExpr) -> Option<&'e TypedExpr> {
        if let Some(found) = self.through_bound(reference, Self::lent_by) {
            return found;
        }
        if let TypedExprKind::Borrow(..) = reference.kind {
            return self.lent_by(reference);
        }
        if let Some(chosen) = self.chosen_values(reference) {
            return chosen.into_iter().find_map(|value| self.lent_by(value));
        }
        self.first_temporary(self.lent.sources.of(reference)?)
    }

    fn through_bound(
        &self,
        value: &'e TypedExpr,
        same_mode: Follow<'w, 'p, 'e>,
    ) -> Option<Option<&'e TypedExpr>> {
        match self.bound.of(value)? {
            BoundLocal::Value(bound) => Some(same_mode(self, bound)),
            BoundLocal::LoopBinder(binder, _) if self.binder_owns_its_value(&binder.ty) => {
                Some(None)
            }
            BoundLocal::LoopBinder(_, start) => Some(same_mode(self, start)),
            BoundLocal::Within(scrutinee) => Some(self.held_in(scrutinee)),
            BoundLocal::Unresolved => Some(None),
        }
    }

    fn binder_owns_its_value(&self, ty: &Type) -> bool {
        !self.lent.program.type_holds_borrow(ty)
    }

    fn chosen_values(&self, value: &'e TypedExpr) -> Option<Vec<&'e TypedExpr>> {
        ReturnedValues::new(self.lent.program).chosen_values(value)
    }

    fn first_temporary(
        &self,
        sources: impl IntoIterator<Item = ResultSource<'e>>,
    ) -> Option<&'e TypedExpr> {
        sources.into_iter().find_map(|source| match source.tie() {
            PositionTie::Lent => self.lent_by(source.argument()),
            PositionTie::Held | PositionTie::HeldBeneath => self.held_in(source.argument()),
        })
    }
}

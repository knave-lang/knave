use super::call_result_sources::CallResultSources;
use super::promoted_constants::PromotedConstants;
use super::result_source::{PassedArgument, ResultSource};
use crate::types::{PositionTie, TypedBinder, TypedExpr, TypedExprKind, TypedProgram};

pub struct LentTemporary<'p> {
    program: &'p TypedProgram,
    sources: CallResultSources<'p>,
    constants: PromotedConstants<'p>,
}

impl<'p> LentTemporary<'p> {
    pub fn new(program: &'p TypedProgram, sources: CallResultSources<'p>) -> Self {
        Self {
            program,
            sources,
            constants: PromotedConstants::new(program),
        }
    }

    pub fn lent_in<'e>(
        &self,
        parameter: &TypedBinder,
        argument: &'e TypedExpr,
    ) -> Option<&'e TypedExpr> {
        self.first_temporary([ResultSource::kept_by_closure(PassedArgument {
            parameter: &parameter.ty,
            argument,
        })])
    }

    fn lent_by<'e>(&self, argument: &'e TypedExpr) -> Option<&'e TypedExpr> {
        let value = match &argument.kind {
            TypedExprKind::Borrow(_, value) => value,
            _ => argument,
        };
        if value.ty.is_reference() {
            return self.borrowed_through(value);
        }
        let owns_its_value = !value.is_place() && !self.constants.is_promoted(value);
        owns_its_value.then_some(value)
    }

    fn held_in<'e>(&self, value: &'e TypedExpr) -> Option<&'e TypedExpr> {
        if value.ty.is_reference() {
            return self.borrowed_through(value);
        }
        if !self.program.type_holds_borrow(&value.ty) {
            return None;
        }
        if let Some(sources) = self.sources.of(value) {
            return self.first_temporary(sources);
        }
        value
            .children()
            .into_iter()
            .find_map(|child| self.held_in(child))
    }

    fn borrowed_through<'e>(&self, reference: &'e TypedExpr) -> Option<&'e TypedExpr> {
        if let TypedExprKind::Borrow(..) = reference.kind {
            return self.lent_by(reference);
        }
        self.first_temporary(self.sources.of(reference)?)
    }

    fn first_temporary<'e>(
        &self,
        sources: impl IntoIterator<Item = ResultSource<'e>>,
    ) -> Option<&'e TypedExpr> {
        sources.into_iter().find_map(|source| match source.tie {
            PositionTie::Lent => self.lent_by(source.argument),
            PositionTie::Held | PositionTie::HeldBeneath => self.held_in(source.argument),
        })
    }
}

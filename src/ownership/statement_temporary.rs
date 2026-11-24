use rustc_hash::FxHashSet;

use super::promoted_constants::PromotedConstants;
use crate::core::BindingId;
use crate::types::{
    CalledFunction, DefinitionTable, ParameterRole, Regions, TypedBinder, TypedExpr, TypedExprKind,
    TypedProgram,
};

pub struct StatementTemporary<'p> {
    program: &'p TypedProgram,
    definitions: &'p DefinitionTable<'p>,
    regions: &'p Regions,
    constants: PromotedConstants<'p>,
}

impl<'p> StatementTemporary<'p> {
    pub fn new(
        program: &'p TypedProgram,
        definitions: &'p DefinitionTable<'p>,
        regions: &'p Regions,
    ) -> Self {
        Self {
            program,
            definitions,
            regions,
            constants: PromotedConstants::new(program),
        }
    }

    pub fn kept_in<'e>(
        &self,
        parameter: &TypedBinder,
        argument: &'e TypedExpr,
    ) -> Option<&'e TypedExpr> {
        if parameter.ty.is_reference() {
            self.lent_by(argument)
        } else {
            self.held_in(argument)
        }
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
        value
            .children()
            .into_iter()
            .find_map(|child| self.held_in(child))
    }

    fn borrowed_through<'e>(&self, reference: &'e TypedExpr) -> Option<&'e TypedExpr> {
        if let TypedExprKind::Borrow(..) = reference.kind {
            return self.lent_by(reference);
        }
        let called = self.definitions.called_function(reference)?;
        let sources = self.result_sources(&called);
        called
            .lambda
            .params
            .iter()
            .zip(called.arguments)
            .filter(|(parameter, _)| sources.contains(&parameter.binding))
            .find_map(|(parameter, argument)| self.kept_in(parameter, &argument.value))
    }

    fn result_sources(&self, called: &CalledFunction) -> FxHashSet<BindingId> {
        let flows = self.regions.of(ParameterRole::FlowsToResult, called.lambda);
        if !flows.is_empty() {
            return flows;
        }
        called
            .lambda
            .params
            .iter()
            .filter(|parameter| {
                parameter.ty.is_reference() || self.program.type_holds_borrow(&parameter.ty)
            })
            .map(|parameter| parameter.binding)
            .collect()
    }
}

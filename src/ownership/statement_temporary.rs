use rustc_hash::FxHashSet;

use super::promoted_constants::PromotedConstants;
use crate::types::{
    CalledFunction, DefinitionTable, ParameterRole, Regions, Type, TypedBinder, TypedExpr,
    TypedExprKind, TypedProgram,
};

enum ResultSources {
    Known(FxHashSet<usize>),
    ByType,
}

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
        self.kept_at(&parameter.ty, argument)
    }

    fn kept_at<'e>(&self, parameter: &Type, argument: &'e TypedExpr) -> Option<&'e TypedExpr> {
        if parameter.is_reference() {
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
        let TypedExprKind::Call(callee, arguments) = &reference.kind else {
            return None;
        };
        let Type::Function(_, parameters, _) = &callee.ty else {
            return None;
        };
        let sources = self.result_sources(reference);
        parameters
            .iter()
            .zip(arguments)
            .enumerate()
            .filter(|(position, (parameter, _))| match &sources {
                ResultSources::Known(positions) => positions.contains(position),
                ResultSources::ByType => self.program.lends_borrow(parameter),
            })
            .find_map(|(_, (parameter, argument))| self.kept_at(parameter, &argument.value))
    }

    fn result_sources(&self, call: &TypedExpr) -> ResultSources {
        match self.definitions.called_function(call) {
            Some(called) => ResultSources::Known(self.flowing_positions(&called)),
            None => ResultSources::ByType,
        }
    }

    fn flowing_positions(&self, called: &CalledFunction) -> FxHashSet<usize> {
        let flows = self.regions.of(ParameterRole::FlowsToResult, called.lambda);
        called
            .lambda
            .params
            .iter()
            .enumerate()
            .filter(|(_, parameter)| flows.contains(&parameter.binding))
            .map(|(position, _)| position)
            .collect()
    }
}

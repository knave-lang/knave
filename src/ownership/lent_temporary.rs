use rustc_hash::FxHashSet;

use super::promoted_constants::PromotedConstants;
use crate::types::{
    CalledFunction, DefinitionTable, LendingTypes, ParameterRole, Regions, Type, TypedArgument,
    TypedBinder, TypedExpr, TypedExprKind, TypedProgram,
};

pub struct LentTemporary<'p> {
    program: &'p TypedProgram,
    definitions: &'p DefinitionTable<'p>,
    regions: &'p Regions,
    constants: PromotedConstants<'p>,
}

impl<'p> LentTemporary<'p> {
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

    pub fn lent_in<'e>(
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
        if let Some(sources) = self.result_sources(value) {
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
        self.first_temporary(self.result_sources(reference)?)
    }

    fn first_temporary<'e>(&self, sources: Vec<(Type, &'e TypedExpr)>) -> Option<&'e TypedExpr> {
        sources
            .into_iter()
            .find_map(|(parameter, argument)| self.kept_at(&parameter, argument))
    }

    fn result_sources<'e>(&self, call: &'e TypedExpr) -> Option<Vec<(Type, &'e TypedExpr)>> {
        let TypedExprKind::Call(callee, arguments) = &call.kind else {
            return None;
        };
        match self.definitions.called_function(call) {
            Some(called) => Some(self.function_sources(&called, arguments)),
            None => self.template_sources(callee, arguments),
        }
    }

    fn function_sources<'e>(
        &self,
        called: &CalledFunction,
        arguments: &'e [TypedArgument],
    ) -> Vec<(Type, &'e TypedExpr)> {
        let flowing = self.flowing_positions(called);
        called
            .lambda
            .params
            .iter()
            .zip(arguments)
            .enumerate()
            .filter(|(position, _)| flowing.contains(position))
            .map(|(_, (parameter, argument))| (parameter.ty.clone(), &argument.value))
            .collect()
    }

    fn template_sources<'e>(
        &self,
        callee: &TypedExpr,
        arguments: &'e [TypedArgument],
    ) -> Option<Vec<(Type, &'e TypedExpr)>> {
        let Type::Function(_, parameters, _) = &callee.ty else {
            return None;
        };
        let receiver = parameters
            .iter()
            .zip(arguments)
            .find(|(parameter, _)| LendingTypes::of(self.program).lends_borrow(parameter))
            .map(|(parameter, argument)| (parameter.clone(), &argument.value));
        Some(receiver.into_iter().collect())
    }

    fn flowing_positions(&self, called: &CalledFunction) -> FxHashSet<usize> {
        let flows = self.regions.of(ParameterRole::FlowsToResult, called.lambda);
        let result = called.lambda.body.tail().map(|tail| &tail.ty);
        let returned_generic = |parameter: &TypedBinder| {
            matches!(parameter.ty, Type::Generic(_)) && Some(&parameter.ty) == result
        };
        let flowing: FxHashSet<usize> = called
            .lambda
            .params
            .iter()
            .enumerate()
            .filter(|(_, parameter)| {
                flows.contains(&parameter.binding) || returned_generic(parameter)
            })
            .map(|(position, _)| position)
            .collect();
        if flowing.is_empty() {
            return self.elided_position(called);
        }
        flowing
    }

    fn elided_position(&self, called: &CalledFunction) -> FxHashSet<usize> {
        let mut lending = called
            .lambda
            .params
            .iter()
            .enumerate()
            .filter(|(_, parameter)| LendingTypes::of(self.program).lends_borrow(&parameter.ty))
            .map(|(position, _)| position);
        match (lending.next(), lending.next()) {
            (Some(only), None) => FxHashSet::from_iter([only]),
            _ => FxHashSet::default(),
        }
    }
}

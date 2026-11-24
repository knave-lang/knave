use super::text::EnumPayloads;
use crate::core::BindingId;
use crate::types::{
    DefinitionTable, FieldOrder, Type, TypedArgument, TypedExpr, TypedExprKind, TypedLeaf,
    TypedProgram,
};

pub struct BuiltParts<'p> {
    definitions: &'p DefinitionTable<'p>,
    payloads: EnumPayloads<'p>,
}

impl<'p> BuiltParts<'p> {
    pub fn new(program: &'p TypedProgram, definitions: &'p DefinitionTable<'p>) -> Self {
        Self {
            definitions,
            payloads: EnumPayloads::new(program),
        }
    }

    pub fn part<'e>(
        &self,
        value: &'e TypedExpr,
        variant: BindingId,
        position: usize,
    ) -> Option<&'e TypedExpr> {
        match &value.kind {
            TypedExprKind::Variant(built, payloads, order) if *built == variant => {
                Self::placed_payload(payloads, order, position)
            }
            TypedExprKind::Call(callee, arguments) => {
                let argument = self.built_argument(callee, variant, position)?;
                arguments
                    .get(argument)
                    .map(|argument: &TypedArgument| &argument.value)
            }
            _ => None,
        }
    }

    fn placed_payload<'e>(
        payloads: &'e [TypedExpr],
        order: &FieldOrder,
        position: usize,
    ) -> Option<&'e TypedExpr> {
        payloads
            .iter()
            .enumerate()
            .find_map(|(written, payload)| (order.field_of(written) == position).then_some(payload))
    }

    fn built_argument(
        &self,
        callee: &TypedExpr,
        variant: BindingId,
        position: usize,
    ) -> Option<usize> {
        let TypedExprKind::Leaf(TypedLeaf::Global(function)) = callee.kind else {
            return None;
        };
        let Type::Function(_, parameters, result) =
            &self.definitions.definition(function)?.scheme.body
        else {
            return None;
        };
        let Type::Named(_, type_arguments) = result.as_ref() else {
            return None;
        };
        let built = type_arguments.get(self.payloads.argument(variant, position)?)?;
        Self::only_parameter_of(parameters, built)
    }

    fn only_parameter_of(parameters: &[Type], built: &Type) -> Option<usize> {
        let Type::Generic(_) = built else {
            return None;
        };
        let mut matching = parameters
            .iter()
            .enumerate()
            .filter(|(_, parameter)| *parameter == built)
            .map(|(index, _)| index);
        let only = matching.next()?;
        matching.next().is_none().then_some(only)
    }
}

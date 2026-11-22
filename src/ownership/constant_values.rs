use crate::core::{BindingId, Literal};
use crate::reader::NumberValue;
use crate::types::{ClauseChoice, TypedExpr, TypedExprKind, TypedLeaf, TypedProgram};

#[derive(Clone, Copy)]
pub(super) struct ConstantValues<'p> {
    program: &'p TypedProgram,
}

impl<'p> ConstantValues<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub fn has_value(&self, operand: &TypedExpr, value: i128) -> bool {
        let operand = self.resolved(operand);
        Self::is_literal_of(operand, value)
            || self
                .negated(operand)
                .is_some_and(|negated| Self::is_literal_of(negated, -value))
    }

    fn resolved<'e>(&self, operand: &'e TypedExpr) -> &'e TypedExpr
    where
        'p: 'e,
    {
        match &operand.kind {
            TypedExprKind::Leaf(TypedLeaf::Global(binding)) => {
                self.definition_value(*binding).unwrap_or(operand)
            }
            _ => operand,
        }
    }

    pub fn definition_value(&self, binding: BindingId) -> Option<&'p TypedExpr> {
        self.program
            .modules
            .iter()
            .flat_map(|module| &module.definitions)
            .find(|definition| definition.binding == binding)
            .map(|definition| &definition.value)
    }

    pub fn negated<'e>(&self, operand: &'e TypedExpr) -> Option<&'e TypedExpr> {
        let TypedExprKind::Call(callee, arguments) = &operand.kind else {
            return None;
        };
        let negates =
            ClauseChoice::new(self.program).read_selected_template(callee, false, |template| {
                template.operation().is_negation()
            });
        arguments
            .first()
            .filter(|_| negates)
            .map(|argument| &argument.value)
    }

    fn is_literal_of(operand: &TypedExpr, value: i128) -> bool {
        matches!(&operand.kind, TypedExprKind::Leaf(TypedLeaf::Literal(Literal::Number(number)))
            if *number.value() == NumberValue::Integer(value))
    }
}

use super::index_problems::IndexProblems;
use super::returned_let::ReturnedLet;
use super::typed::{TypedArgument, TypedControl, TypedExpr, TypedExprKind, TypedLeaf};
use crate::core::{Conversion, Literal};
use crate::diagnostic::Diagnostics;
use crate::reader::{Number, NumberValue};
use crate::source::Span;

pub(super) struct NegativeIndexLiterals<'c> {
    conversions: &'c [(usize, Conversion)],
}

impl<'c> NegativeIndexLiterals<'c> {
    pub(super) fn new(conversions: &'c [(usize, Conversion)]) -> Self {
        Self { conversions }
    }

    pub(super) fn problems(&self, arguments: &[TypedArgument]) -> Diagnostics {
        let mut problems = Diagnostics::default();
        let checked = self
            .conversions
            .iter()
            .filter(|(_, conversion)| conversion.rejects_negative_literal())
            .filter_map(|(position, _)| arguments.get(*position));
        for argument in checked {
            Self::report_tails(&argument.value, &mut problems);
        }
        problems
    }

    pub(super) fn in_values(values: &[TypedExpr]) -> Diagnostics {
        let mut problems = Diagnostics::default();
        for value in values {
            Self::report_tails(value, &mut problems);
        }
        problems
    }

    fn report_tails(value: &TypedExpr, problems: &mut Diagnostics) {
        if let Some(bound) = ReturnedLet::new(value).value() {
            return Self::report_tails(bound, problems);
        }
        match &value.kind {
            TypedExprKind::Leaf(TypedLeaf::Literal(Literal::Number(number))) => {
                Self::report_literal(number, value.span, problems);
            }
            TypedExprKind::Control(control) => {
                for tail in Self::tails(control) {
                    Self::report_tails(tail, problems);
                }
            }
            _ => {}
        }
    }

    fn tails(control: &TypedControl) -> Vec<&TypedExpr> {
        match control {
            TypedControl::Block(block) => block.tail().into_iter().collect(),
            TypedControl::If(_, then, otherwise) => vec![then, otherwise],
            TypedControl::Match(matched) => matched
                .arms
                .iter()
                .filter_map(|arm| arm.body.tail())
                .collect(),
        }
    }

    fn report_literal(number: &Number, span: Span, problems: &mut Diagnostics) {
        if matches!(number.value(), NumberValue::Integer(integer) if *integer < 0) {
            problems.push(IndexProblems::negative_index(number.text(), span));
        }
    }
}

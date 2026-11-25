use super::argument_passing::{PassedArgument, Passing};
use super::expression_walk::ExpressionWalk;
use super::holdings::Holdings;
use super::lender::Lender;
use super::passed_holdings::PassedHoldings;
use super::point_id::PointId;
use super::walked::Walked;
use crate::types::TypedArgument;

pub struct ArgumentValues<'w> {
    arguments: &'w [TypedArgument],
    passings: Vec<&'static dyn Passing>,
    overwritten: Option<usize>,
}

pub struct EvaluatedArguments {
    pub end: PointId,
    pub held: Vec<Holdings>,
}

impl<'w> ArgumentValues<'w> {
    pub fn new(arguments: &'w [TypedArgument], passings: Vec<&'static dyn Passing>) -> Self {
        Self {
            arguments,
            passings,
            overwritten: None,
        }
    }

    pub fn overwriting(self, overwritten: Option<usize>) -> Self {
        Self {
            overwritten,
            ..self
        }
    }

    pub fn evaluate(&self, walk: &mut dyn ExpressionWalk<'w>, at: PointId) -> EvaluatedArguments {
        let mut evaluated = EvaluatedArguments {
            end: at,
            held: Vec::new(),
        };
        for (position, argument) in self.arguments.iter().enumerate() {
            let walked = match self.passings.get(position) {
                _ if self.overwritten == Some(position) => Walked::at(evaluated.end),
                Some(passing) if passing.takes_the_place() => {
                    walk.place_root(&argument.value, evaluated.end)
                }
                _ => walk.expr(&argument.value, evaluated.end),
            };
            evaluated.end = walked.end;
            evaluated.held.push(walked.held);
        }
        evaluated
    }

    pub fn pass(&self, held: Vec<Holdings>, lender: &mut Lender<'_>) -> Vec<PassedHoldings> {
        self.arguments
            .iter()
            .zip(&self.passings)
            .zip(held)
            .map(|((argument, passing), held)| {
                let passed = passing.pass(PassedArgument::new(argument, held), lender);
                lender.receive(&passed.all(), argument.value.span);
                passed
            })
            .collect()
    }
}

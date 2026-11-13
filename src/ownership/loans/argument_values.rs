use super::argument_passing::{PassedArgument, Passing};
use super::expression_walk::ExpressionWalk;
use super::holdings::Holdings;
use super::lender::Lender;
use super::point_id::PointId;
use crate::types::TypedArgument;

pub struct ArgumentValues<'w> {
    arguments: &'w [TypedArgument],
    passings: Vec<&'static dyn Passing>,
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
        }
    }

    pub fn evaluate(&self, walk: &mut dyn ExpressionWalk<'w>, at: PointId) -> EvaluatedArguments {
        let mut evaluated = EvaluatedArguments {
            end: at,
            held: Vec::new(),
        };
        for (position, argument) in self.arguments.iter().enumerate() {
            let walked = match self.passings.get(position) {
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

    pub fn pass(&self, held: Vec<Holdings>, lender: &mut Lender<'_>) -> Vec<Holdings> {
        self.arguments
            .iter()
            .zip(&self.passings)
            .zip(held)
            .map(|((argument, passing), held)| {
                let passed = passing.pass(PassedArgument::new(argument, held), lender);
                lender.receive(&passed);
                passed
            })
            .collect()
    }
}

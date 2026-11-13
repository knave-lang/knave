use super::argument_passing::Passing;
use super::argument_values::ArgumentValues;
use super::expression_walk::ExpressionWalk;
use super::point_id::PointId;
use super::walked::Walked;
use crate::types::{TypedArgument, TypedBinder, TypedLambda};

pub struct LoopWalk<'c, 'w> {
    walk: &'c mut dyn ExpressionWalk<'w>,
}

impl<'c, 'w> LoopWalk<'c, 'w> {
    pub fn new(walk: &'c mut dyn ExpressionWalk<'w>) -> Self {
        Self { walk }
    }

    pub fn repeated(
        mut self,
        lambda: &'w TypedLambda,
        arguments: &'w [TypedArgument],
        at: PointId,
    ) -> Walked {
        let started = self.bound(&lambda.params, arguments, at);
        let head = self.walk.builder().join(&[started]);
        self.walk.builder().loops().enter(head, &lambda.params);
        let body = self.walk.block(&lambda.body, head);
        self.walk.builder().loops().leave();
        body
    }

    pub fn recur(mut self, arguments: &'w [TypedArgument], at: PointId) -> Walked {
        let Some(parameters) = self.walk.builder().loops().innermost_parameters() else {
            return Walked::at(at);
        };
        let again = self.bound(parameters, arguments, at);
        self.walk.builder().loops().return_to_innermost(again);
        Walked::at(again)
    }

    fn bound(
        &mut self,
        parameters: &'w [TypedBinder],
        arguments: &'w [TypedArgument],
        at: PointId,
    ) -> PointId {
        let rules = self.walk.rules();
        let passings = arguments
            .iter()
            .zip(parameters)
            .map(|(argument, parameter)| <dyn Passing>::of(argument, Some(&parameter.ty), rules))
            .collect();
        let values = ArgumentValues::new(arguments, passings);
        let evaluated = values.evaluate(self.walk, at);
        let builder = self.walk.builder();
        let point = builder.join(&[evaluated.end]);
        let passed = values.pass(evaluated.held, &mut builder.lender(point));
        for (parameter, passed) in parameters.iter().zip(passed) {
            builder.define(point, parameter, &passed.all());
        }
        point
    }
}

use super::argument_passing::Passing;
use super::argument_values::ArgumentValues;
use super::expression_walk::ExpressionWalk;
use super::holdings::Holdings;
use super::loop_heads::LoopHead;
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
        let head = self.walk.join(&[started]);
        self.walk.loops().enter(LoopHead {
            head,
            parameters: &lambda.params,
        });
        let body = self.walk.block(&lambda.body, head);
        self.walk.loops().leave();
        body
    }

    pub fn recur(mut self, arguments: &'w [TypedArgument], at: PointId) -> Walked {
        if let Some(innermost) = self.walk.loops().innermost() {
            let again = self.bound(innermost.parameters, arguments, at);
            self.walk.link(again, innermost.head);
        }
        Walked::at(self.walk.join(&[]))
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
        let point = self.walk.join(&[evaluated.end]);
        let passed: Vec<Holdings> = values.pass(evaluated.held, &mut self.walk.lender(point));
        for (parameter, held) in parameters.iter().zip(passed) {
            self.walk.define(point, parameter, &held);
        }
        point
    }
}

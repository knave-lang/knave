use super::argument_passing::Passing;
use super::argument_values::ArgumentValues;
use super::call_site::CallSite;
use super::expression_walk::ExpressionWalk;
use super::point_id::PointId;
use super::walked::Walked;

pub struct CallWalk<'c, 'w> {
    walk: &'c mut dyn ExpressionWalk<'w>,
    site: CallSite<'w>,
}

impl<'c, 'w> CallWalk<'c, 'w> {
    pub fn new(walk: &'c mut dyn ExpressionWalk<'w>, site: CallSite<'w>) -> Self {
        Self { walk, site }
    }

    pub fn walk(self, at: PointId) -> Walked {
        let rules = self.walk.rules();
        let called = self.walk.expr(self.site.callee(), at);
        let passings = self
            .site
            .arguments()
            .iter()
            .enumerate()
            .map(|(position, argument)| {
                <dyn Passing>::of(argument, self.site.parameter(position), rules)
            })
            .collect();
        let values = ArgumentValues::new(self.site.arguments(), passings);
        let evaluated = values.evaluate(self.walk, called.end);
        let point = self.walk.join(&[evaluated.end]);
        let passed = values.pass(evaluated.held, &mut self.walk.lender(point));
        Walked::holding(point, rules.result_of(&self.site, passed))
    }
}

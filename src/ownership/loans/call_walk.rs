use super::argument_passing::Passing;
use super::argument_values::ArgumentValues;
use super::call_site::CallSite;
use super::expression_walk::ExpressionWalk;
use super::holdings::Holdings;
use super::loan_place::LoanPlace;
use super::passed_holdings::PassedHoldings;
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

    pub fn walk(mut self, at: PointId) -> Walked {
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
        let point = self.walk.builder().join(&[evaluated.end]);
        let passed = values.pass(evaluated.held, &mut self.walk.builder().lender(point));
        self.store_into_targets(&passed);
        Walked::holding(point, self.site.result_holdings(passed))
    }

    fn store_into_targets(&mut self, passed: &[PassedHoldings]) {
        let stores = self.walk.rules().callees().stores(self.site.callee());
        for (target, values) in stores.stores() {
            let stored = values
                .iter()
                .filter_map(|value| passed.get(*value))
                .fold(Holdings::default(), |held, value| held.merged(value.all()));
            self.store_into(target, stored);
        }
    }

    fn store_into(&mut self, target: usize, stored: Holdings) {
        let Some(argument) = self.site.arguments().get(target) else {
            return;
        };
        let Some(place) = LoanPlace::of(&argument.value) else {
            return;
        };
        let held = if argument.value.ty.is_reference() {
            stored.within()
        } else {
            stored
        };
        self.walk.builder().values().store(place.root(), &held);
    }
}

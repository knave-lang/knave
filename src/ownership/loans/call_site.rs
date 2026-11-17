use super::called_parameter::CalledParameter;
use super::holdings::Holdings;
use super::loan_rules::LoanRules;
use super::passed_holdings::PassedHoldings;
use crate::types::{
    Borrow, PositionTie, TemplateTies, Type, TypedArgument, TypedExpr, TypedExprKind, TypedLambda,
};

pub struct CallSite<'c> {
    callee: &'c TypedExpr,
    arguments: &'c [TypedArgument],
    ties: Option<TemplateTies>,
}

impl<'c> CallSite<'c> {
    pub fn new(
        callee: &'c TypedExpr,
        arguments: &'c [TypedArgument],
        rules: &LoanRules<'_>,
    ) -> Self {
        Self {
            callee,
            arguments,
            ties: rules.ties(callee),
        }
    }

    pub fn repeated_lambda(&self) -> Option<&'c TypedLambda> {
        match &self.callee.kind {
            TypedExprKind::Fn(lambda) => Some(lambda),
            _ => None,
        }
    }

    pub fn callee(&self) -> &'c TypedExpr {
        self.callee
    }

    pub fn arguments(&self) -> &'c [TypedArgument] {
        self.arguments
    }

    pub fn parameter(&self, position: usize) -> CalledParameter<'c> {
        let ty = self.parameter_type(position);
        if self.stores_the_reference(position) {
            CalledParameter::storing(ty)
        } else {
            CalledParameter::untied(ty)
        }
    }

    fn stores_the_reference(&self, position: usize) -> bool {
        self.ties
            .as_ref()
            .is_some_and(|ties| ties.stores_the_reference(position, self.uncollapsed()))
    }

    fn uncollapsed(&self) -> impl Iterator<Item = usize> + 'c {
        self.arguments
            .iter()
            .enumerate()
            .filter(|(_, argument)| !matches!(argument.borrow, Borrow::Collapse(_)))
            .map(|(position, _)| position)
    }

    fn parameter_type(&self, position: usize) -> Option<&'c Type> {
        match &self.callee.ty {
            Type::Function(_, parameters, _) => parameters.get(position),
            _ => None,
        }
    }

    pub fn result_holdings(&self, passed: Vec<PassedHoldings>) -> Holdings {
        let Some(ties) = &self.ties else {
            return Holdings::default();
        };
        passed
            .into_iter()
            .enumerate()
            .fold(Holdings::default(), |held, (position, passed)| {
                match ties.tie_at(position) {
                    Some(PositionTie::Lent) => held.merged(passed.all()),
                    Some(PositionTie::HeldBeneath) => held.merged(passed.held().referent()),
                    Some(PositionTie::Held) => held.merged(passed.held().clone()),
                    None => held,
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::CallSite;
    use crate::ownership::lambda_source::LambdaSource;
    use crate::ownership::loans::loan_rules::LoanRules;
    use crate::types::{TypedExpr, TypedExprKind};

    struct SiteFixture;

    impl SiteFixture {
        fn with_site<T>(
            call: &TypedExpr,
            rules: &LoanRules<'_>,
            read: impl FnOnce(CallSite<'_>) -> T,
        ) -> T {
            match &call.kind {
                TypedExprKind::Call(callee, arguments) => {
                    read(CallSite::new(callee, arguments, rules))
                }
                _ => panic!("the value is a call"),
            }
        }
    }

    #[test]
    fn a_call_names_its_parameters_and_tells_a_repeated_lambda() {
        let source = LambdaSource::new(
            "(defn main ()\n  (local v (vec 1))\n  (local r (at v 0))\n  (local n (len v))\n  (println r n))",
        );
        let rules = LoanRules::of(source.program());
        SiteFixture::with_site(source.bound("r"), &rules, |site| {
            assert!(
                site.parameter(0)
                    .ty()
                    .is_some_and(|parameter| parameter.is_reference())
            );
            assert!(site.parameter(2).ty().is_none());
            assert!(site.repeated_lambda().is_none());
        });
    }
}

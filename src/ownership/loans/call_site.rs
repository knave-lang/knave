use super::holdings::Holdings;
use crate::types::{TemplateBorrows, Type, TypedArgument, TypedExpr, TypedExprKind, TypedLambda};

pub struct CallSite<'c> {
    call: &'c TypedExpr,
    callee: &'c TypedExpr,
    arguments: &'c [TypedArgument],
}

impl<'c> CallSite<'c> {
    pub fn new(call: &'c TypedExpr, callee: &'c TypedExpr, arguments: &'c [TypedArgument]) -> Self {
        Self {
            call,
            callee,
            arguments,
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

    pub fn parameter(&self, position: usize) -> Option<&'c Type> {
        match &self.callee.ty {
            Type::Function(_, parameters, _) => parameters.get(position),
            _ => None,
        }
    }

    pub fn result_holdings(&self, passed: Vec<Holdings>, templates: &TemplateBorrows) -> Holdings {
        let is_template = templates.read_template(self.callee, false, |_| true);
        if !self.returns_a_borrow() || !is_template {
            return Holdings::default();
        }
        let mut lent = passed
            .into_iter()
            .enumerate()
            .filter(|(position, _)| self.lends_through(*position))
            .map(|(_, held)| held);
        match (lent.next(), lent.next()) {
            (Some(only), None) => only,
            _ => Holdings::default(),
        }
    }

    fn lends_through(&self, position: usize) -> bool {
        self.parameter(position)
            .is_none_or(|parameter| parameter.holds_reference())
    }

    fn returns_a_borrow(&self) -> bool {
        self.call.ty.holds_reference()
    }
}

#[cfg(test)]
mod tests {
    use super::CallSite;
    use crate::ownership::lambda_source::LambdaSource;
    use crate::types::{TypedExpr, TypedExprKind};

    struct SiteFixture;

    impl SiteFixture {
        fn with_site<T>(call: &TypedExpr, read: impl FnOnce(CallSite<'_>) -> T) -> T {
            match &call.kind {
                TypedExprKind::Call(callee, arguments) => {
                    read(CallSite::new(call, callee, arguments))
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
        SiteFixture::with_site(source.bound("r"), |site| {
            assert!(site.returns_a_borrow());
            assert!(
                site.parameter(0)
                    .is_some_and(|parameter| parameter.is_reference())
            );
            assert!(site.parameter(2).is_none());
            assert!(site.repeated_lambda().is_none());
        });
    }
}

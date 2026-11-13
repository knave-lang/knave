use crate::types::{Type, TypedArgument, TypedExpr, TypedExprKind, TypedLambda};

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

    pub fn returns_a_borrow(&self) -> bool {
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
    fn a_call_names_its_parameters_and_whether_it_returns_a_borrow() {
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
        SiteFixture::with_site(source.bound("n"), |site| assert!(!site.returns_a_borrow()));
    }
}

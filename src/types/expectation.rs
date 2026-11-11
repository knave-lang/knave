use super::problems::MismatchContext;
use super::ty::Type;

pub(super) struct Expectation<'a> {
    pub(super) expected: &'a Type,
    pub(super) context: MismatchContext,
}

impl<'a> Expectation<'a> {
    pub(super) fn new(expected: &'a Type, context: MismatchContext) -> Self {
        Self { expected, context }
    }

    pub(super) fn plain(expected: &'a Type) -> Self {
        Self::new(expected, MismatchContext::Plain)
    }
}

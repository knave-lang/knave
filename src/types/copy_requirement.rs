use super::bound_origins::BoundOrigins;
use super::ty::{Bound, Type};
use super::unifier::Unifier;
use crate::core::BindingId;

pub(super) struct CopyRequirement<'u> {
    copy_trait: Option<BindingId>,
    unifier: &'u mut Unifier,
}

impl<'u> CopyRequirement<'u> {
    pub(super) fn new(copy_trait: Option<BindingId>, unifier: &'u mut Unifier) -> Self {
        Self {
            copy_trait,
            unifier,
        }
    }

    pub(super) fn require(&mut self, value: &Type, origins: BoundOrigins) {
        let Type::Var(var) = self.unifier.shallow(value) else {
            return;
        };
        if let Some(copy) = self.copy_trait {
            self.unifier
                .add_bound(var, Bound::plain(copy).from_origins(origins));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Type;
    use crate::types::bound_origins::BoundOrigins;
    use crate::types::passing_fixture::PassingFixture;

    #[test]
    fn a_variable_gets_the_copy_bound() {
        let mut fixture = PassingFixture::new();
        let open = fixture.unifier().fresh();
        fixture
            .copy_requirement()
            .require(&open, BoundOrigins::none());
        let Type::Var(var) = open else {
            unreachable!("fresh makes a variable")
        };
        let copy = fixture.copy_trait();
        assert!(
            fixture
                .unifier()
                .bounds(var)
                .iter()
                .any(|bound| bound.binding == copy)
        );
    }
}

use super::ty::{Type, TypeVar};

pub(super) struct OccursCheck<Shallow: Fn(&Type) -> Type> {
    sought: TypeVar,
    shallow: Shallow,
}

impl<Shallow: Fn(&Type) -> Type> OccursCheck<Shallow> {
    pub(super) fn new(sought: TypeVar, shallow: Shallow) -> Self {
        Self { sought, shallow }
    }

    pub(super) fn finds_in(&self, value: &Type) -> bool {
        match value {
            Type::Var(_) => self.finds_behind_var(value),
            other => other.parts().into_iter().any(|part| self.finds_in(part)),
        }
    }

    fn finds_behind_var(&self, var: &Type) -> bool {
        match (self.shallow)(var) {
            Type::Var(last) => last == self.sought,
            bound => self.finds_in(&bound),
        }
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::*;

    struct OccursFixture {
        bindings: FxHashMap<TypeVar, Type>,
        sought: TypeVar,
    }

    impl OccursFixture {
        const SOUGHT: usize = 0;
        const OTHER: usize = 1;

        fn new() -> Self {
            Self {
                bindings: FxHashMap::default(),
                sought: TypeVar::new(Self::SOUGHT),
            }
        }

        fn other() -> TypeVar {
            TypeVar::new(Self::OTHER)
        }

        fn bind_other(&mut self, value: Type) {
            self.bindings.insert(Self::other(), value);
        }

        fn shallow(&self, value: &Type) -> Type {
            match value {
                Type::Var(var) => match self.bindings.get(var) {
                    Some(bound) => self.shallow(bound),
                    None => value.clone(),
                },
                other => other.clone(),
            }
        }

        fn finds_sought_in(&self, value: &Type) -> bool {
            OccursCheck::new(self.sought, |value: &Type| self.shallow(value)).finds_in(value)
        }
    }

    #[test]
    fn a_variable_merged_into_the_sought_one_is_found() {
        let mut fixture = OccursFixture::new();
        fixture.bind_other(Type::Var(fixture.sought));
        assert!(fixture.finds_sought_in(&Type::shared(Type::Var(OccursFixture::other()))));
    }

    #[test]
    fn a_variable_bound_to_a_type_holding_the_sought_one_is_found() {
        let mut fixture = OccursFixture::new();
        fixture.bind_other(Type::shared(Type::Var(fixture.sought)));
        assert!(fixture.finds_sought_in(&Type::Var(OccursFixture::other())));
    }

    #[test]
    fn an_unbound_other_variable_is_not_found() {
        let fixture = OccursFixture::new();
        assert!(!fixture.finds_sought_in(&Type::shared(Type::Var(OccursFixture::other()))));
    }
}

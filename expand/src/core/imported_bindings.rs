use super::BindingId;

#[derive(Clone, Copy, Debug)]
pub struct ImportedBindings {
    first: BindingId,
    second: Option<BindingId>,
}

impl ImportedBindings {
    pub fn one(binding: BindingId) -> Self {
        Self {
            first: binding,
            second: None,
        }
    }

    pub fn two(first: BindingId, second: BindingId) -> Self {
        Self {
            first,
            second: Some(second),
        }
    }

    pub fn found(bindings: [Option<BindingId>; 2]) -> Option<Self> {
        match bindings {
            [Some(first), Some(second)] => Some(Self::two(first, second)),
            [Some(only), None] | [None, Some(only)] => Some(Self::one(only)),
            [None, None] => None,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = BindingId> + use<> {
        std::iter::once(self.first).chain(self.second)
    }
}

#[cfg(test)]
mod tests {
    use super::ImportedBindings;
    use crate::core::BindingId;

    #[test]
    fn a_name_that_binds_one_namespace_holds_one_binding() {
        let bindings = ImportedBindings::one(BindingId::from_index(3));
        assert_eq!(
            bindings.iter().collect::<Vec<_>>(),
            [BindingId::from_index(3)]
        );
    }

    #[test]
    fn a_name_that_binds_both_namespaces_holds_both_in_order() {
        let bindings = ImportedBindings::two(BindingId::from_index(3), BindingId::from_index(5));
        assert_eq!(
            bindings.iter().collect::<Vec<_>>(),
            [BindingId::from_index(3), BindingId::from_index(5)]
        );
    }
}

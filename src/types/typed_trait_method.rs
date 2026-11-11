use super::receiver_kind::ReceiverKind;
use crate::core::BindingId;

#[derive(Clone)]
pub struct TypedTraitMethod {
    pub binding: BindingId,
    pub receiver: Option<ReceiverKind>,
    pub default: Option<BindingId>,
    pub returns_self: bool,
}

impl TypedTraitMethod {
    pub fn declaration(&self) -> BindingId {
        self.default.unwrap_or(self.binding)
    }
}

#[cfg(test)]
mod tests {
    use super::TypedTraitMethod;
    use crate::core::BindingId;

    struct MethodFixture;

    impl MethodFixture {
        const DECLARED: BindingId = BindingId::from_index(0);
        const DEFAULT: BindingId = BindingId::from_index(1);

        fn method(default: Option<BindingId>) -> TypedTraitMethod {
            TypedTraitMethod {
                binding: Self::DECLARED,
                receiver: None,
                default,
                returns_self: false,
            }
        }
    }

    #[test]
    fn a_method_with_a_default_body_is_declared_by_its_default() {
        let method = MethodFixture::method(Some(MethodFixture::DEFAULT));
        assert_eq!(method.declaration(), MethodFixture::DEFAULT);
    }

    #[test]
    fn a_method_without_a_default_body_is_declared_by_its_binding() {
        let method = MethodFixture::method(None);
        assert_eq!(method.declaration(), MethodFixture::DECLARED);
    }
}

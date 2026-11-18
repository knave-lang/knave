use super::ReceiverKind;
use crate::core::{BindingId, ReferenceKind, TypeExpr, TypeExprKind};

pub(super) struct MethodSignature<'a> {
    signature: &'a TypeExpr,
    self_variable: BindingId,
}

impl<'a> MethodSignature<'a> {
    pub(super) fn new(signature: &'a TypeExpr, self_variable: BindingId) -> Self {
        Self {
            signature,
            self_variable,
        }
    }

    pub(super) fn receiver(&self) -> Option<ReceiverKind> {
        let TypeExprKind::Function(_, parameters, _) = self.signature.kind() else {
            return None;
        };
        parameters
            .first()
            .and_then(|parameter| self.parameter_receiver(parameter))
    }

    pub(super) fn returns_self(&self) -> bool {
        let TypeExprKind::Function(_, _, result) = self.signature.kind() else {
            return false;
        };
        matches!(
            result.kind(),
            TypeExprKind::Named(binding, arguments)
                if *binding == self.self_variable && arguments.is_empty()
        )
    }

    fn parameter_receiver(&self, parameter: &TypeExpr) -> Option<ReceiverKind> {
        match parameter.kind() {
            TypeExprKind::Named(binding, arguments) => {
                self.named_receiver(*binding, arguments, ReceiverKind::Owned)
            }
            TypeExprKind::Reference(kind, _, inner) => self.reference_receiver(*kind, inner),
            _ => None,
        }
    }

    fn reference_receiver(&self, kind: ReferenceKind, inner: &TypeExpr) -> Option<ReceiverKind> {
        let TypeExprKind::Named(binding, arguments) = inner.kind() else {
            return None;
        };
        let receiver = match kind {
            ReferenceKind::Shared => ReceiverKind::Shared,
            ReferenceKind::Mutable => ReceiverKind::Mutable,
        };
        self.named_receiver(*binding, arguments, receiver)
    }

    fn named_receiver(
        &self,
        binding: BindingId,
        arguments: &[TypeExpr],
        receiver: ReceiverKind,
    ) -> Option<ReceiverKind> {
        (binding == self.self_variable && arguments.is_empty()).then_some(receiver)
    }
}

#[cfg(test)]
mod tests {
    use super::MethodSignature;
    use crate::core::{BindingId, ReferenceKind, TypeExpr};
    use crate::types::ReceiverKind;
    use crate::types::written_type_fixture::WrittenTypeFixture as Fixture;

    struct SignatureFixture {
        fixture: Fixture,
        self_variable: BindingId,
    }

    impl SignatureFixture {
        fn new() -> Self {
            let mut fixture = Fixture::new();
            let self_variable = fixture.type_variable("self");
            Self {
                fixture,
                self_variable,
            }
        }

        fn self_type(&self) -> TypeExpr {
            Fixture::named(self.self_variable, Vec::new())
        }

        fn receiver_of(&self, first: TypeExpr) -> Option<ReceiverKind> {
            let signature = Fixture::function(vec![first], Fixture::integer());
            MethodSignature::new(&signature, self.self_variable).receiver()
        }

        fn returns_self(&self, result: TypeExpr) -> bool {
            let signature = Fixture::function(Vec::new(), result);
            MethodSignature::new(&signature, self.self_variable).returns_self()
        }
    }

    #[test]
    fn a_self_parameter_receives_by_value() {
        let fixture = SignatureFixture::new();
        assert_eq!(
            fixture.receiver_of(fixture.self_type()),
            Some(ReceiverKind::Owned)
        );
    }

    #[test]
    fn references_to_self_receive_by_reference() {
        let fixture = SignatureFixture::new();
        let shared = Fixture::reference(ReferenceKind::Shared, fixture.self_type());
        let mutable = Fixture::reference(ReferenceKind::Mutable, fixture.self_type());
        assert_eq!(fixture.receiver_of(shared), Some(ReceiverKind::Shared));
        assert_eq!(fixture.receiver_of(mutable), Some(ReceiverKind::Mutable));
    }

    #[test]
    fn self_applied_to_arguments_is_no_receiver() {
        let fixture = SignatureFixture::new();
        let applied = Fixture::named(fixture.self_variable, vec![Fixture::integer()]);
        assert_eq!(fixture.receiver_of(applied), None);
    }

    #[test]
    fn another_type_first_is_no_receiver() {
        let mut fixture = SignatureFixture::new();
        let other = fixture.fixture.type_variable("other");
        assert_eq!(fixture.receiver_of(Fixture::named(other, Vec::new())), None);
        assert_eq!(fixture.receiver_of(Fixture::integer()), None);
    }

    #[test]
    fn a_signature_that_is_no_function_has_no_receiver_and_no_self_result() {
        let fixture = SignatureFixture::new();
        let signature = fixture.self_type();
        let method = MethodSignature::new(&signature, fixture.self_variable);
        assert_eq!(method.receiver(), None);
        assert!(!method.returns_self());
    }

    #[test]
    fn a_function_without_parameters_has_no_receiver() {
        let fixture = SignatureFixture::new();
        let signature = Fixture::function(Vec::new(), Fixture::integer());
        assert_eq!(
            MethodSignature::new(&signature, fixture.self_variable).receiver(),
            None
        );
    }

    #[test]
    fn only_a_bare_self_result_returns_self() {
        let fixture = SignatureFixture::new();
        let applied = Fixture::named(fixture.self_variable, vec![Fixture::integer()]);
        assert!(fixture.returns_self(fixture.self_type()));
        assert!(!fixture.returns_self(applied));
        assert!(!fixture.returns_self(Fixture::integer()));
    }
}

use std::rc::Rc;

use super::method_signature::MethodSignature;
use super::{RustTrait, TraitMethodInfo};
use crate::core::{BindingId, RustTraitDeclaration};

pub(super) struct CollectedRustTrait<'a> {
    pub(super) declaration: &'a RustTraitDeclaration,
    pub(super) rust_trait: RustTrait,
    pub(super) methods: Vec<(BindingId, TraitMethodInfo)>,
}

impl<'a> CollectedRustTrait<'a> {
    pub(super) fn new(declaration: &'a RustTraitDeclaration, rust_trait: RustTrait) -> Self {
        let methods = declaration
            .methods
            .iter()
            .map(|method| {
                let signature = MethodSignature::new(&method.signature, declaration.self_variable);
                let info = TraitMethodInfo {
                    trait_binding: declaration.binding,
                    rust_name: Rc::clone(&declaration.path),
                    receiver: signature.receiver(),
                };
                (method.binding, info)
            })
            .collect();
        Self {
            declaration,
            rust_trait,
            methods,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CollectedRustTrait;
    use crate::core::ReferenceKind;
    use crate::types::trait_fixture::TraitFixture;
    use crate::types::written_type_fixture::WrittenTypeFixture as Fixture;
    use crate::types::{ReceiverKind, RustTrait};

    #[test]
    fn a_rust_trait_enters_its_methods_as_synthetic_with_their_receivers() {
        let mut fixture = TraitFixture::new();
        let self_variable = fixture.types.type_variable("self");
        let receiver = Fixture::reference(
            ReferenceKind::Mutable,
            Fixture::named(self_variable, Vec::new()),
        );
        let method =
            fixture.trait_method(Fixture::function(vec![receiver], Fixture::integer()), None);
        let method_binding = method.binding;
        let mut declaration = fixture.rust_trait_declaration(vec![method]);
        declaration.self_variable = self_variable;
        let rust_trait = RustTrait::unreached(&declaration);
        fixture.enter_rust_trait(CollectedRustTrait::new(&declaration, rust_trait));
        let checker = &fixture.types.checker;
        let info = checker.declarations.traits().methods.get(&method_binding);
        let info = info.expect("the method is entered");
        assert_eq!(info.trait_binding, declaration.binding);
        assert_eq!(info.receiver, Some(ReceiverKind::Mutable));
        assert!(checker.synthetic_bindings.contains(&method_binding));
        assert!(
            checker
                .declarations
                .traits()
                .table
                .get(declaration.binding)
                .is_some()
        );
    }
}

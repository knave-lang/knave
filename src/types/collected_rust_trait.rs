use std::rc::Rc;

use rustc_hash::FxHashSet;

use super::method_signature::MethodSignature;
use super::trait_declarations::TraitDeclarations;
use super::{RustTrait, TraitMethodInfo};
use crate::core::{BindingId, RustTraitDeclaration};

pub(super) struct CollectedRustTrait<'a> {
    declaration: &'a RustTraitDeclaration,
    rust_trait: RustTrait,
}

impl<'a> CollectedRustTrait<'a> {
    pub(super) fn new(declaration: &'a RustTraitDeclaration, rust_trait: RustTrait) -> Self {
        Self {
            declaration,
            rust_trait,
        }
    }

    pub(super) fn enter(
        self,
        traits: &mut TraitDeclarations,
        synthetic: &mut FxHashSet<BindingId>,
    ) {
        let declaration = self.declaration;
        traits.table.add(declaration.binding, self.rust_trait);
        traits.rust_paths.record(declaration);
        for method in &declaration.methods {
            let receiver =
                MethodSignature::new(&method.signature, declaration.self_variable).receiver();
            synthetic.insert(method.binding);
            traits.methods.insert(
                method.binding,
                TraitMethodInfo {
                    trait_binding: declaration.binding,
                    rust_name: Rc::clone(&declaration.path),
                    receiver,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CollectedRustTrait;
    use crate::core::ReferenceKind;
    use crate::types::written_type_fixture::WrittenTypeFixture as Fixture;
    use crate::types::{ReceiverKind, RustTrait};

    #[test]
    fn a_rust_trait_enters_its_methods_as_synthetic_with_their_receivers() {
        let mut fixture = Fixture::new();
        let self_variable = fixture.type_variable("self");
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
        let checker = &fixture.checker;
        let info = checker.declarations.traits.methods.get(&method_binding);
        let info = info.expect("the method is entered");
        assert_eq!(info.trait_binding, declaration.binding);
        assert_eq!(info.receiver, Some(ReceiverKind::Mutable));
        assert!(checker.synthetic_bindings.contains(&method_binding));
        assert!(
            checker
                .declarations
                .traits
                .table
                .get(declaration.binding)
                .is_some()
        );
    }
}

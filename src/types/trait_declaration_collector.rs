use super::collected_trait::CollectedTrait;
use super::method_signature::MethodSignature;
use super::selfless_method::SelflessMethod;
use super::ty::Bound;
use super::user_trait::UserTrait;
use super::written_type_lowering::WrittenTypeLowering;
use super::{LoweredRows, RustTrait, TypedAssociatedType, TypedTrait, TypedTraitMethod};
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;

pub(super) struct TraitDeclarationCollector<'c> {
    lowering: WrittenTypeLowering<'c>,
}

impl<'c> TraitDeclarationCollector<'c> {
    pub(super) fn new(lowering: WrittenTypeLowering<'c>) -> Self {
        Self { lowering }
    }

    pub(super) fn collect_user_trait(
        &mut self,
        declaration: UserTrait<'_>,
    ) -> Result<CollectedTrait, Diagnostic> {
        let associated = Self::associated_types(declaration);
        let supertraits = self.supertraits(declaration)?;
        let rust_trait = RustTrait::written(
            declaration.rust_name.clone(),
            associated.first().map(|associated| associated.name.clone()),
            LoweredRows::new(Vec::new(), supertraits.clone()),
        );
        let definition = TypedTrait {
            binding: declaration.binding,
            module: declaration.module,
            rust_name: declaration.rust_name.clone(),
            supertraits,
            associated,
            methods: Self::methods(declaration),
        };
        Ok(CollectedTrait::new(
            rust_trait,
            definition,
            Self::selfless_methods(declaration),
        ))
    }

    fn associated_types(declaration: UserTrait<'_>) -> Vec<TypedAssociatedType> {
        declaration
            .associated
            .iter()
            .map(|associated| TypedAssociatedType {
                binding: associated.binding,
                name: associated.name.clone(),
            })
            .collect()
    }

    fn supertraits(&mut self, declaration: UserTrait<'_>) -> Result<Vec<Bound>, Diagnostic> {
        declaration
            .supertraits
            .iter()
            .map(|bound| self.lowering.lower_type_bound(bound, &Default::default()))
            .collect()
    }

    fn methods(declaration: UserTrait<'_>) -> Vec<TypedTraitMethod> {
        declaration
            .methods
            .iter()
            .map(|method| {
                let signature = MethodSignature::new(&method.signature, declaration.self_variable);
                TypedTraitMethod {
                    binding: method.binding,
                    receiver: signature.receiver(),
                    default: method.default,
                    returns_self: signature.returns_self(),
                }
            })
            .collect()
    }

    fn selfless_methods(declaration: UserTrait<'_>) -> Vec<(BindingId, SelflessMethod)> {
        declaration
            .methods
            .iter()
            .filter_map(|method| {
                SelflessMethod::of(method, declaration.binding, declaration.self_variable)
                    .map(|selfless| (method.binding, selfless))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::core::{ReferenceKind, TraitAssociatedType};
    use crate::types::ReceiverKind;
    use crate::types::trait_fixture::TraitFixture;
    use crate::types::ty::Bound;
    use crate::types::user_trait::UserTrait;
    use crate::types::written_type_fixture::WrittenTypeFixture as Fixture;

    #[test]
    fn a_user_trait_enters_its_definition_methods_and_synthetic_bindings() {
        let mut fixture = TraitFixture::new();
        let shown = fixture.types.named_trait("shown");
        let display = fixture.types.named_trait("display");
        let self_variable = fixture.types.type_variable("self");
        let receiver = Fixture::reference(
            ReferenceKind::Shared,
            Fixture::named(self_variable, Vec::new()),
        );
        let default = fixture.default_method();
        let methods = [fixture.trait_method(
            Fixture::function(vec![receiver], Fixture::integer()),
            Some(default),
        )];
        let supertraits = [Fixture::bounded(display, &[display]).bounds.remove(0)];
        let rust_name: Rc<str> = "Shown".into();
        let collected = fixture
            .trait_collector()
            .collect_user_trait(UserTrait {
                module: crate::core::ModuleId::new(0),
                binding: shown,
                rust_name: &rust_name,
                self_variable,
                supertraits: &supertraits,
                associated: &[],
                methods: &methods,
            })
            .expect("the trait collects");
        fixture.enter_trait(collected);
        let checker = &fixture.types.checker;
        let traits = &checker.declarations.traits;
        let definition = &traits.definitions[0];
        assert_eq!(definition.supertraits, vec![Bound::plain(display)]);
        assert_eq!(definition.methods[0].receiver, Some(ReceiverKind::Shared));
        assert_eq!(definition.methods[0].default, Some(default));
        let info = traits
            .methods
            .get(&methods[0].binding)
            .expect("the method is entered");
        assert_eq!(info.trait_binding, shown);
        assert_eq!(info.receiver, Some(ReceiverKind::Shared));
        assert!(traits.table.get(shown).is_some());
        assert!(checker.synthetic_bindings.contains(&methods[0].binding));
        assert!(checker.synthetic_bindings.contains(&default));
    }

    #[test]
    fn a_method_without_self_is_recorded_as_selfless() {
        let mut fixture = TraitFixture::new();
        let shown = fixture.types.named_trait("shown");
        let self_variable = fixture.types.type_variable("self");
        let methods =
            [fixture.trait_method(Fixture::function(Vec::new(), Fixture::integer()), None)];
        let rust_name: Rc<str> = "Shown".into();
        let associated = [TraitAssociatedType {
            binding: fixture.types.associated_type("item"),
            name: "Item".into(),
        }];
        let collected = fixture
            .trait_collector()
            .collect_user_trait(UserTrait {
                module: crate::core::ModuleId::new(0),
                binding: shown,
                rust_name: &rust_name,
                self_variable,
                supertraits: &[],
                associated: &associated,
                methods: &methods,
            })
            .expect("the trait collects");
        fixture.enter_trait(collected);
        let checker = &fixture.types.checker;
        assert!(
            checker
                .checking_facts
                .selfless_methods
                .get(methods[0].binding)
                .is_some()
        );
        let definition = &checker.declarations.traits.definitions[0];
        assert_eq!(definition.associated.len(), 1);
        assert!(!definition.methods[0].returns_self);
    }

    #[test]
    fn a_method_returning_self_is_marked_as_returning_self() {
        let mut fixture = TraitFixture::new();
        let shown = fixture.types.named_trait("shown");
        let self_variable = fixture.types.type_variable("self");
        let methods = [fixture.trait_method(
            Fixture::function(Vec::new(), Fixture::named(self_variable, Vec::new())),
            None,
        )];
        let rust_name: Rc<str> = "Shown".into();
        let collected = fixture
            .trait_collector()
            .collect_user_trait(UserTrait {
                module: crate::core::ModuleId::new(0),
                binding: shown,
                rust_name: &rust_name,
                self_variable,
                supertraits: &[],
                associated: &[],
                methods: &methods,
            })
            .expect("the trait collects");
        fixture.enter_trait(collected);
        let definition = &fixture.types.checker.declarations.traits.definitions[0];
        assert!(definition.methods[0].returns_self);
        let selfless = &fixture.types.checker.checking_facts.selfless_methods;
        assert!(selfless.get(methods[0].binding).is_none());
    }
}

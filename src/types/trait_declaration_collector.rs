use super::collected_trait::CollectedTrait;
use super::method_signature::MethodSignature;
use super::user_trait::UserTrait;
use super::written_type_lowering::WrittenTypeLowering;
use super::{LoweredRows, RustTrait, TypedAssociatedType, TypedTrait, TypedTraitMethod};
use crate::diagnostic::Diagnostic;

pub(super) struct TraitDeclarationCollector<'c> {
    lowering: WrittenTypeLowering<'c>,
}

impl<'c> TraitDeclarationCollector<'c> {
    pub(super) fn new(lowering: WrittenTypeLowering<'c>) -> Self {
        Self { lowering }
    }

    pub(super) fn collect_user_trait<'a>(
        &mut self,
        declaration: UserTrait<'a>,
    ) -> Result<CollectedTrait<'a>, Diagnostic> {
        let associated: Vec<TypedAssociatedType> = declaration
            .associated
            .iter()
            .map(|associated| TypedAssociatedType {
                binding: associated.binding,
                name: associated.name.clone(),
            })
            .collect();
        let supertraits = declaration
            .supertraits
            .iter()
            .map(|bound| self.lowering.lower_type_bound(bound, &Default::default()))
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        let rust_trait = RustTrait::written(
            declaration.rust_name.clone(),
            associated.first().map(|associated| associated.name.clone()),
            LoweredRows::new(Vec::new(), supertraits.clone()),
        );
        let methods = declaration
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
            .collect();
        let definition = TypedTrait {
            binding: declaration.binding,
            module: declaration.module,
            rust_name: declaration.rust_name.clone(),
            supertraits,
            associated,
            methods,
        };
        Ok(CollectedTrait::new(declaration, rust_trait, definition))
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::core::{ReferenceKind, TraitAssociatedType};
    use crate::types::ReceiverKind;
    use crate::types::ty::Bound;
    use crate::types::user_trait::UserTrait;
    use crate::types::written_type_fixture::WrittenTypeFixture as Fixture;

    #[test]
    fn a_user_trait_enters_its_definition_methods_and_synthetic_bindings() {
        let mut fixture = Fixture::new();
        let shown = fixture.named_trait("shown");
        let display = fixture.named_trait("display");
        let self_variable = fixture.type_variable("self");
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
        let checker = &fixture.checker;
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
        let mut fixture = Fixture::new();
        let shown = fixture.named_trait("shown");
        let self_variable = fixture.type_variable("self");
        let methods =
            [fixture.trait_method(Fixture::function(Vec::new(), Fixture::integer()), None)];
        let rust_name: Rc<str> = "Shown".into();
        let associated = [TraitAssociatedType {
            binding: fixture.associated_type("item"),
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
        let checker = &fixture.checker;
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
}

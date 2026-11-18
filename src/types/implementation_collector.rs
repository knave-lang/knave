use std::rc::Rc;

use super::collected_impl::CollectedImpl;
use super::declared_traits::DeclaredTraits;
use super::impl_item::ImplItem;
use super::lowered_impl::LoweredImpl;
use super::ty::{Scheme, Type};
use super::{Implementor, TypedImplMethod, TypedTraitImpl};
use crate::core::BindingId;

pub(super) struct ImplementationCollector<'a> {
    traits: DeclaredTraits<'a>,
}

impl<'a> ImplementationCollector<'a> {
    pub(super) fn new(traits: DeclaredTraits<'a>) -> Self {
        Self { traits }
    }

    pub(super) fn collect(&self, item: ImplItem<'_>, lowered: LoweredImpl) -> CollectedImpl {
        let (implementor, params, associated_values) = lowered.into_parts();
        let entry = Implementor {
            scheme: Scheme {
                lifetimes: Vec::new(),
                params: params.clone(),
                body: implementor.clone(),
            },
            argument: self.argument(item.trait_binding, &associated_values),
        };
        let definition = TypedTraitImpl {
            trait_binding: item.trait_binding,
            implementor,
            params,
            associated: self.associated(item.trait_binding, associated_values),
            implementor_span: item.implementor.span(),
            module: item.module,
            methods: self.methods(item),
        };
        CollectedImpl::new(entry, definition)
    }

    fn argument(&self, trait_binding: BindingId, values: &[(BindingId, Type)]) -> Option<Type> {
        self.traits
            .definition(trait_binding)
            .is_some_and(|definition| !definition.associated.is_empty())
            .then(|| values.first().map(|(_, value)| value.clone()))
            .flatten()
    }

    fn methods(&self, item: ImplItem<'_>) -> Vec<TypedImplMethod> {
        item.methods
            .iter()
            .map(|method| TypedImplMethod {
                binding: method.binding,
                receiver: self.traits.receiver(method.trait_method),
            })
            .collect()
    }

    fn associated(
        &self,
        trait_binding: BindingId,
        values: Vec<(BindingId, Type)>,
    ) -> Vec<(BindingId, Rc<str>, Type)> {
        values
            .into_iter()
            .map(|(binding, value)| (binding, self.associated_name(trait_binding, binding), value))
            .collect()
    }

    fn associated_name(&self, trait_binding: BindingId, associated: BindingId) -> Rc<str> {
        self.traits
            .definition(trait_binding)
            .and_then(|trait_definition| {
                trait_definition
                    .associated
                    .iter()
                    .find(|candidate| candidate.binding == associated)
            })
            .map_or_else(|| Rc::from(""), |candidate| candidate.name.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{BindingId, ModuleId, PrimitiveType};
    use crate::types::Type;
    use crate::types::declarations_fixture::DeclarationsFixture;
    use crate::types::impl_item::ImplItem;
    use crate::types::lowered_impl::LoweredImpl;
    use crate::types::written_type_fixture::WrittenTypeFixture;

    const SOURCE: &str = "(trait container (type item) (size (fn ((ref self)) usize)))\n\
                          (trait producer (type output) (produce (fn ((ref self)) usize)))\n";

    struct CollectorFixture {
        declarations: DeclarationsFixture,
    }

    impl CollectorFixture {
        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(SOURCE),
            }
        }

        fn enter_impl_of(&mut self, trait_name: &str) {
            let trait_binding = self.declarations.trait_named(trait_name);
            let associated = self.associated_type_of(trait_binding);
            let implementor = WrittenTypeFixture::integer();
            let item = ImplItem {
                module: ModuleId::new(0),
                trait_binding,
                implementor: &implementor,
                associated: &[],
                methods: &[],
            };
            let usize_type = Type::Primitive(PrimitiveType::Usize);
            let lowered = LoweredImpl::new(
                Type::Primitive(PrimitiveType::I64),
                Vec::new(),
                vec![(associated, usize_type)],
            );
            let collected = self
                .declarations
                .implementation_collector()
                .collect(item, lowered);
            self.declarations.enter_impl(collected);
        }

        fn associated_type_of(&self, trait_binding: BindingId) -> BindingId {
            let traits = self.declarations.declared_traits();
            let definition = traits
                .definition(trait_binding)
                .expect("the trait is defined");
            definition.associated[0].binding
        }
    }

    #[test]
    fn an_impl_names_its_associated_type_from_its_own_trait() {
        let mut fixture = CollectorFixture::new();
        fixture.enter_impl_of("producer");
        let implementation = fixture
            .declarations
            .traits()
            .impls()
            .last()
            .expect("an impl");
        assert_eq!(&*implementation.associated[0].1, "output");
    }

    #[test]
    fn an_entered_impl_adds_its_implementor_to_the_trait() {
        let mut fixture = CollectorFixture::new();
        let producer = fixture.declarations.trait_named("producer");
        let before = fixture.declarations.traits().impls().len();
        fixture.enter_impl_of("producer");
        assert_eq!(fixture.declarations.traits().impls().len(), before + 1);
        assert!(
            fixture
                .declarations
                .traits()
                .table()
                .get(producer)
                .is_some()
        );
    }
}

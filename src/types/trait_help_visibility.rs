use super::declarations::Declarations;
use super::declared_types::DeclaredTypes;
use super::derivable_traits::DerivableTraits;
use super::ty::Type;
use crate::core::BindingId;

pub(super) struct TraitHelpVisibility<'a> {
    declarations: &'a Declarations,
}

impl<'a> TraitHelpVisibility<'a> {
    pub(super) fn new(declarations: &'a Declarations) -> Self {
        Self { declarations }
    }

    pub(super) fn shows(&self, trait_binding: BindingId, implementor: &Type) -> bool {
        if !DerivableTraits::new(self.declarations.traits()).is_structural(trait_binding) {
            let program_type =
                DeclaredTypes::new(self.declarations.types()).declares_type(implementor);
            return program_type || !self.declares_methods(trait_binding);
        }
        self.lacks_source_form(implementor)
            || self
                .blocking_types(implementor)
                .any(|blocking| self.lacks_source_form(blocking))
    }

    fn blocking_types(&self, implementor: &Type) -> impl Iterator<Item = &'a Type> + 'a {
        let types = self.declarations.types();
        let blocked = match implementor {
            Type::Named(binding, _) => match types.structs.get(*binding) {
                Some(definition) => definition.blocked.as_slice(),
                None => types
                    .enums
                    .get(*binding)
                    .map_or(&[][..], |definition| definition.blocked.as_slice()),
            },
            _ => &[],
        };
        blocked.iter().map(|(_, _, ty)| ty)
    }

    fn lacks_source_form(&self, ty: &Type) -> bool {
        let rust_types = &self.declarations.types().rust_types;
        let opaque = matches!(ty, Type::Named(binding, _) if rust_types.is_opaque(*binding));
        opaque
            || matches!(ty, Type::Function(..))
            || ty
                .parts()
                .into_iter()
                .any(|part| self.lacks_source_form(part))
    }

    fn declares_methods(&self, trait_binding: BindingId) -> bool {
        self.declarations
            .traits()
            .methods
            .values()
            .any(|method| method.trait_binding == trait_binding)
    }
}

#[cfg(test)]
mod tests {
    use super::TraitHelpVisibility;
    use crate::core::PrimitiveType;
    use crate::types::Type;
    use crate::types::declarations_fixture::DeclarationsFixture;

    const SOURCE: &str = "
        (defstruct score :points i64)
        (trait shown (show (fn ((ref self)) usize)))
        (trait marked)
        (defn main () (do))";

    struct VisibilityFixture {
        declarations: DeclarationsFixture,
    }

    impl VisibilityFixture {
        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(SOURCE),
            }
        }

        fn shows(&self, trait_name: &str, implementor: &Type) -> bool {
            let trait_binding = self.declarations.trait_named(trait_name);
            TraitHelpVisibility::new(self.declarations.declarations())
                .shows(trait_binding, implementor)
        }

        fn named(&self, name: &str) -> Type {
            Type::Named(self.declarations.type_named(name), Vec::new())
        }
    }

    #[test]
    fn a_trait_with_methods_shows_its_help_on_a_program_type() {
        let fixture = VisibilityFixture::new();
        assert!(fixture.shows("shown", &fixture.named("score")));
    }

    #[test]
    fn a_trait_with_methods_hides_its_help_on_a_prelude_type() {
        let fixture = VisibilityFixture::new();
        assert!(!fixture.shows("shown", &Type::Primitive(PrimitiveType::Bool)));
    }

    #[test]
    fn a_trait_without_methods_shows_its_help_on_any_type() {
        let fixture = VisibilityFixture::new();
        assert!(fixture.shows("marked", &Type::Primitive(PrimitiveType::Bool)));
    }
}

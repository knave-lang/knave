use super::ReceiverKind;
use super::declarations::Declarations;
use super::declared_types::DeclaredTypes;
use super::display::TypeDisplay;
use super::impl_overlap::ImplementorOverlap;
use super::problems::TypeProblems;
use super::ty::Type;
use crate::core::{BindingArena, BindingId, ImplMethod};
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub(super) struct ImplementationCheck<'a> {
    bindings: &'a BindingArena,
    declarations: &'a Declarations,
    display: TypeDisplay<'a>,
}

impl<'a> ImplementationCheck<'a> {
    pub(super) fn new(
        bindings: &'a BindingArena,
        declarations: &'a Declarations,
        display: TypeDisplay<'a>,
    ) -> Self {
        Self {
            bindings,
            declarations,
            display,
        }
    }

    pub(super) fn rejection(
        &self,
        trait_binding: BindingId,
        implementor: &Type,
        span: Span,
    ) -> Option<Diagnostic> {
        self.foreign_implementor(trait_binding, implementor, span)
            .or_else(|| self.second_impl(trait_binding, implementor, span))
    }

    pub(super) fn unsized_problems(
        &self,
        implementor: &Type,
        span: Span,
        methods: &[ImplMethod],
    ) -> Vec<Diagnostic> {
        if !DeclaredTypes::new(&self.declarations.types).is_unsized(implementor) {
            return Vec::new();
        }
        methods
            .iter()
            .filter(|method| self.receives_owned(method.trait_method))
            .map(|method| {
                let implementor_name = self.display.show(implementor);
                let method_name = self.bindings.get(method.trait_method).name().to_string();
                TypeProblems::unsized_implementor(&implementor_name, &method_name, span)
            })
            .collect()
    }

    fn receives_owned(&self, trait_method: BindingId) -> bool {
        self.declarations
            .traits
            .methods
            .get(&trait_method)
            .is_some_and(|info| info.receiver == Some(ReceiverKind::Owned))
    }

    fn foreign_implementor(
        &self,
        trait_binding: BindingId,
        implementor: &Type,
        span: Span,
    ) -> Option<Diagnostic> {
        let rust_paths = &self.declarations.traits.rust_paths;
        let program_type = DeclaredTypes::new(&self.declarations.types).declares_type(implementor);
        if rust_paths.written(trait_binding).is_none() || program_type {
            return None;
        }
        let implementor_name = self.display.show(implementor);
        let trait_name = self.bindings.get(trait_binding).name().to_string();
        Some(TypeProblems::foreign_implementor(
            &trait_name,
            &implementor_name,
            span,
        ))
    }

    fn second_impl(
        &self,
        trait_binding: BindingId,
        implementor: &Type,
        span: Span,
    ) -> Option<Diagnostic> {
        let first = self
            .declarations
            .traits
            .impls
            .iter()
            .find(|implementation| {
                implementation.trait_binding == trait_binding
                    && ImplementorOverlap::overlaps(&implementation.implementor, implementor)
            })?;
        let first = TypeProblems::first_impl(first.implementor_span);
        let implementor_name = self.display.show(implementor);
        let trait_name = self.bindings.get(trait_binding).name().to_string();
        Some(TypeProblems::second_impl(
            &implementor_name,
            &trait_name,
            span,
            first,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::core::PrimitiveType;
    use crate::source::{SourceId, Span};
    use crate::types::Type;
    use crate::types::declarations_fixture::DeclarationsFixture;

    const SOURCE: &str = "(trait numbered (number (fn ((ref self)) i64)))\n\
                          (trait viewed (view (fn ((ref self)) i64)))\n\
                          (impl numbered i64 (number (n) 1))\n";

    struct CheckFixture {
        declarations: DeclarationsFixture,
    }

    impl CheckFixture {
        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(SOURCE),
            }
        }

        fn rejects(&self, trait_name: &str, implementor: PrimitiveType) -> bool {
            let trait_binding = self.declarations.trait_named(trait_name);
            let span = Span::new(SourceId::new(0), 0..0);
            self.declarations
                .implementation_check()
                .rejection(trait_binding, &Type::Primitive(implementor), span)
                .is_some()
        }
    }

    #[test]
    fn a_second_impl_for_the_same_type_is_rejected() {
        assert!(CheckFixture::new().rejects("numbered", PrimitiveType::I64));
    }

    #[test]
    fn an_impl_for_another_type_is_not_rejected() {
        assert!(!CheckFixture::new().rejects("numbered", PrimitiveType::Bool));
    }

    #[test]
    fn an_impl_of_another_trait_for_the_same_type_is_not_rejected() {
        assert!(!CheckFixture::new().rejects("viewed", PrimitiveType::I64));
    }
}

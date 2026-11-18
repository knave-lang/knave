use super::ReceiverKind;
use super::declarations::Declarations;
use super::declared_traits::DeclaredTraits;
use super::declared_types::DeclaredTypes;
use super::display::TypeDisplay;
use super::impl_item::ImplItem;
use super::implementation_verdict::ImplementationVerdict;
use super::problems::TypeProblems;
use super::rust_trait_paths::RustTraitPaths;
use super::ty::Type;
use crate::core::{BindingArena, BindingId, ImplMethod};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::source::Span;

pub(super) struct ImplementationCheck<'a> {
    bindings: &'a BindingArena,
    types: DeclaredTypes<'a>,
    traits: DeclaredTraits<'a>,
    rust_paths: &'a RustTraitPaths,
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
            types: DeclaredTypes::new(declarations.types()),
            traits: DeclaredTraits::new(declarations.traits()),
            rust_paths: &declarations.traits().rust_paths,
            display,
        }
    }

    pub(super) fn verdict(&self, item: ImplItem<'_>, implementor: &Type) -> ImplementationVerdict {
        let span = item.implementor.span();
        match self.rejection(item.trait_binding, implementor, span) {
            Some(rejection) => ImplementationVerdict::Rejected(rejection),
            None => ImplementationVerdict::Accepted(self.unsized_problems(
                implementor,
                span,
                item.methods,
            )),
        }
    }

    fn rejection(
        &self,
        trait_binding: BindingId,
        implementor: &Type,
        span: Span,
    ) -> Option<Diagnostic> {
        self.foreign_implementor(trait_binding, implementor, span)
            .or_else(|| self.second_impl(trait_binding, implementor, span))
    }

    fn unsized_problems(
        &self,
        implementor: &Type,
        span: Span,
        methods: &[ImplMethod],
    ) -> Diagnostics {
        if !self.types.is_unsized(implementor) {
            return Diagnostics::default();
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
        self.traits.receiver(trait_method) == Some(ReceiverKind::Owned)
    }

    fn foreign_implementor(
        &self,
        trait_binding: BindingId,
        implementor: &Type,
        span: Span,
    ) -> Option<Diagnostic> {
        let program_type = self.types.declares_type(implementor);
        if self.rust_paths.written(trait_binding).is_none() || program_type {
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
        let first = self.traits.overlapping_impl(trait_binding, implementor)?;
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
    use crate::core::{BindingId, PrimitiveType};
    use crate::types::Type;
    use crate::types::declarations_fixture::DeclarationsFixture;
    use crate::types::implementation_verdict::ImplementationVerdict;

    const SOURCE: &str = "(trait numbered (number (fn ((ref self)) i64)))\n\
                          (trait viewed (view (fn ((ref self)) i64)))\n\
                          (trait consumed (take (fn (self) i64)))\n\
                          (impl numbered i64 (number (n) 1))\n\
                          (impl viewed bool (view (flag) 0))\n\
                          (impl consumed bool (take (flag) 0))\n";

    struct CheckFixture {
        declarations: DeclarationsFixture,
    }

    impl CheckFixture {
        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(SOURCE),
            }
        }

        fn verdict_on(&self, trait_name: &str, implementor: Type) -> ImplementationVerdict {
            let item = self.declarations.impl_item(trait_name);
            self.declarations
                .implementation_check()
                .verdict(item, &implementor)
        }

        fn integer() -> Type {
            Type::Primitive(PrimitiveType::I64)
        }

        fn text() -> Type {
            Type::Primitive(PrimitiveType::Str)
        }

        fn rejected(verdict: &ImplementationVerdict) -> bool {
            matches!(verdict, ImplementationVerdict::Rejected(_))
        }

        fn problems(verdict: &ImplementationVerdict) -> usize {
            match verdict {
                ImplementationVerdict::Accepted(problems) => problems.len(),
                ImplementationVerdict::Rejected(_) => panic!("the impl is accepted"),
            }
        }

        fn vector(&self) -> Type {
            let vec = self.declarations.rust_type_named("vec");
            Type::Named(vec, vec![Self::integer()])
        }

        fn display(&self) -> BindingId {
            self.declarations.trait_named("display")
        }
    }

    #[test]
    fn a_second_impl_for_the_same_type_is_rejected() {
        let fixture = CheckFixture::new();
        let verdict = fixture.verdict_on("numbered", CheckFixture::integer());
        assert!(CheckFixture::rejected(&verdict));
    }

    #[test]
    fn an_impl_for_another_type_is_accepted() {
        let fixture = CheckFixture::new();
        let verdict = fixture.verdict_on("numbered", Type::Primitive(PrimitiveType::Bool));
        assert_eq!(CheckFixture::problems(&verdict), 0);
    }

    #[test]
    fn an_impl_of_another_trait_for_the_same_type_is_accepted() {
        let fixture = CheckFixture::new();
        let verdict = fixture.verdict_on("viewed", CheckFixture::integer());
        assert!(!CheckFixture::rejected(&verdict));
    }

    #[test]
    fn an_unsized_implementor_reports_a_method_that_takes_self() {
        let fixture = CheckFixture::new();
        let verdict = fixture.verdict_on("consumed", CheckFixture::text());
        assert_eq!(CheckFixture::problems(&verdict), 1);
    }

    #[test]
    fn an_unsized_implementor_may_borrow_self() {
        let fixture = CheckFixture::new();
        let verdict = fixture.verdict_on("viewed", CheckFixture::text());
        assert_eq!(CheckFixture::problems(&verdict), 0);
    }

    #[test]
    fn a_sized_implementor_may_take_self() {
        let fixture = CheckFixture::new();
        let verdict = fixture.verdict_on("consumed", CheckFixture::integer());
        assert_eq!(CheckFixture::problems(&verdict), 0);
    }

    #[test]
    fn a_foreign_type_cannot_implement_a_rust_trait() {
        let fixture = CheckFixture::new();
        let check = fixture.declarations.implementation_check();
        let mut item = fixture.declarations.impl_item("numbered");
        item.trait_binding = fixture.display();
        let verdict = check.verdict(item, &fixture.vector());
        assert!(CheckFixture::rejected(&verdict));
    }
}

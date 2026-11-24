use super::{ExpressionForm, VariableTyping};
use crate::core::{BindingId, BindingKind};
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::expectation::Expectation;
use crate::types::problems::TypeProblems;
use crate::types::ty::Type;
use crate::types::typed::{TypedExpr, TypedExprKind, TypedLeaf};
use crate::types::variant_arm_reference::VariantArmReference;

pub(in crate::types) struct VariableReference {
    binding: BindingId,
    span: Span,
}

impl VariableReference {
    pub(in crate::types) fn new(binding: BindingId, span: Span) -> Self {
        Self { binding, span }
    }

    fn type_variant_arm<C: VariableTyping>(
        &self,
        context: &mut C,
        reference: VariantArmReference,
    ) -> Result<TypedExpr, Diagnostic> {
        match reference.outer {
            Some(outer) => Self::new(outer, self.span).type_against(context, None),
            None => {
                let name = context.binding_name(self.binding);
                let variant = context.binding_name(reference.variant);
                Err(TypeProblems::variant_arm_value(&name, &variant, self.span))
            }
        }
    }

    fn type_global<C: VariableTyping>(
        &self,
        context: &mut C,
        name: &str,
    ) -> Result<TypedExpr, Diagnostic> {
        if context.is_unchecked(self.binding) {
            return Err(TypeProblems::compile_time_only(name, self.span));
        }
        if let Some(problem) = context.selfless_method_problem(self.binding, name, self.span) {
            return Err(problem);
        }
        Ok(self.typed(
            TypedLeaf::Global(self.binding),
            context.global_type(self.binding, self.span),
        ))
    }

    fn typed(&self, leaf: TypedLeaf, ty: Type) -> TypedExpr {
        TypedExpr {
            kind: TypedExprKind::Leaf(leaf),
            ty,
            span: self.span,
        }
    }
}

impl<C: VariableTyping> ExpressionForm<C> for VariableReference {
    fn type_against(
        &self,
        context: &mut C,
        _hint: Option<Expectation>,
    ) -> Result<TypedExpr, Diagnostic> {
        if let Some(reference) = context.variant_arm(self.binding) {
            return self.type_variant_arm(context, reference);
        }
        let name = context.binding_name(self.binding);
        match context.binding_kind(self.binding) {
            BindingKind::Local => Ok(self.typed(
                TypedLeaf::Local(self.binding),
                context.local_type(self.binding),
            )),
            BindingKind::Global(_) => self.type_global(context, &name),
            _ => Err(TypeProblems::compile_time_only(&name, self.span)),
        }
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::VariableReference;
    use crate::core::{BindingId, BindingKind, ModuleId, PrimitiveType};
    use crate::diagnostic::Diagnostic;
    use crate::source::Span;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{
        Bindings, ExpressionForm, GlobalReferences, LocalTypes, VariantArms,
    };
    use crate::types::problems::TypeProblems;
    use crate::types::ty::Type;
    use crate::types::typed::TypedExpr;
    use crate::types::variant_arm_reference::VariantArmReference;

    struct VariableFixture {
        kinds: FxHashMap<BindingId, BindingKind>,
        arms: FxHashMap<BindingId, VariantArmReference>,
        unchecked: bool,
        selfless: bool,
    }

    impl VariableFixture {
        const VARIABLE: BindingId = BindingId::from_index(1);
        const OUTER: BindingId = BindingId::from_index(2);
        const VARIANT: BindingId = BindingId::from_index(3);

        fn of(kind: BindingKind) -> Self {
            Self {
                kinds: FxHashMap::from_iter([(Self::VARIABLE, kind)]),
                arms: FxHashMap::default(),
                unchecked: false,
                selfless: false,
            }
        }

        fn global() -> BindingKind {
            BindingKind::Global(ModuleId::new(0))
        }

        fn in_variant_arm(mut self, outer: Option<BindingId>) -> Self {
            let reference = VariantArmReference {
                outer,
                variant: Self::VARIANT,
            };
            self.arms.insert(Self::VARIABLE, reference);
            self.kinds.insert(Self::OUTER, BindingKind::Local);
            self
        }

        fn local_type() -> Type {
            Type::Primitive(PrimitiveType::U8)
        }

        fn global_type() -> Type {
            Type::Primitive(PrimitiveType::I64)
        }

        fn reference(&mut self) -> Result<TypedExpr, Diagnostic> {
            VariableReference::new(Self::VARIABLE, FormFixtures::span()).type_against(self, None)
        }

        fn type_of(&mut self) -> Option<Type> {
            self.reference().ok().map(|typed| typed.ty)
        }
    }

    impl Bindings for VariableFixture {
        fn binding_name(&self, _binding: BindingId) -> String {
            "name".to_string()
        }

        fn binding_kind(&self, binding: BindingId) -> BindingKind {
            self.kinds[&binding]
        }
    }

    impl VariantArms for VariableFixture {
        fn variant_arm(&self, binding: BindingId) -> Option<VariantArmReference> {
            self.arms.get(&binding).copied()
        }
    }

    impl LocalTypes for VariableFixture {
        fn local_type(&self, _binding: BindingId) -> Type {
            Self::local_type()
        }
    }

    impl GlobalReferences for VariableFixture {
        fn is_unchecked(&self, _binding: BindingId) -> bool {
            self.unchecked
        }

        fn selfless_method_problem(
            &self,
            _binding: BindingId,
            _name: &str,
            span: Span,
        ) -> Option<Diagnostic> {
            self.selfless.then(|| TypeProblems::quote_at_runtime(span))
        }

        fn global_type(&mut self, _binding: BindingId, _span: Span) -> Type {
            Self::global_type()
        }
    }

    #[test]
    fn a_local_has_its_local_type() {
        let ty = VariableFixture::of(BindingKind::Local).type_of();
        assert_eq!(ty, Some(VariableFixture::local_type()));
    }

    #[test]
    fn a_global_has_its_global_type() {
        let ty = VariableFixture::of(VariableFixture::global()).type_of();
        assert_eq!(ty, Some(VariableFixture::global_type()));
    }

    #[test]
    fn an_unchecked_global_is_refused() {
        let mut fixture = VariableFixture::of(VariableFixture::global());
        fixture.unchecked = true;
        assert!(fixture.reference().is_err());
    }

    #[test]
    fn a_selfless_method_is_refused() {
        let mut fixture = VariableFixture::of(VariableFixture::global());
        fixture.selfless = true;
        assert!(fixture.reference().is_err());
    }

    #[test]
    fn a_compile_time_binding_is_refused() {
        let mut fixture = VariableFixture::of(BindingKind::Macro(ModuleId::new(0)));
        assert!(fixture.reference().is_err());
    }

    #[test]
    fn a_variant_arm_binding_reads_its_outer_binding() {
        let mut fixture = VariableFixture::of(VariableFixture::global())
            .in_variant_arm(Some(VariableFixture::OUTER));
        assert_eq!(fixture.type_of(), Some(VariableFixture::local_type()));
    }

    #[test]
    fn a_variant_arm_binding_without_an_outer_binding_is_refused() {
        let mut fixture = VariableFixture::of(BindingKind::Local).in_variant_arm(None);
        assert!(fixture.reference().is_err());
    }
}

use std::rc::Rc;

use super::{ExpressionForm, SchemeInstantiation, TemplateSchemes};
use crate::core::RustTemplate;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::expectation::Expectation;
use crate::types::typed::{TypedExpr, TypedExprKind, TypedLeaf};

pub(in crate::types) struct InlineRust<'e> {
    template: &'e Rc<RustTemplate>,
    span: Span,
}

impl<'e> InlineRust<'e> {
    pub(in crate::types) fn new(template: &'e Rc<RustTemplate>, span: Span) -> Self {
        Self { template, span }
    }
}

impl<C: TemplateSchemes + SchemeInstantiation> ExpressionForm<C> for InlineRust<'_> {
    fn type_against(
        &self,
        context: &mut C,
        _hint: Option<Expectation>,
    ) -> Result<TypedExpr, Diagnostic> {
        let scheme = context.lower_template(self.template)?;
        context.register_inline(&self.template.template, &scheme);
        Ok(TypedExpr {
            kind: TypedExprKind::Leaf(TypedLeaf::Rust(self.template.clone())),
            ty: context.instantiate(&scheme, self.span),
            span: self.span,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::InlineRust;
    use crate::core::{PrimitiveType, RustTemplate, TypeExpr, TypeExprKind};
    use crate::diagnostic::Diagnostic;
    use crate::source::Span;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{ExpressionForm, SchemeInstantiation, TemplateSchemes};
    use crate::types::ty::{Scheme, Type};

    #[derive(Default)]
    struct TemplateFixture {
        registered: Vec<Rc<str>>,
    }

    impl TemplateFixture {
        fn template() -> Rc<RustTemplate> {
            Rc::new(RustTemplate {
                template: Rc::from("1"),
                type_expr: TypeExpr::new(
                    TypeExprKind::Primitive(PrimitiveType::I64),
                    FormFixtures::span(),
                ),
                clauses: Vec::new(),
            })
        }

        fn instantiated() -> Type {
            Type::Primitive(PrimitiveType::U8)
        }

        fn type_template(&mut self) -> Option<Type> {
            let template = Self::template();
            InlineRust::new(&template, FormFixtures::span())
                .type_against(self, None)
                .ok()
                .map(|typed| typed.ty)
        }
    }

    impl TemplateSchemes for TemplateFixture {
        fn lower_template(&mut self, _template: &RustTemplate) -> Result<Scheme, Diagnostic> {
            Ok(Scheme::monomorphic(Type::Primitive(PrimitiveType::I64)))
        }

        fn register_inline(&mut self, text: &Rc<str>, _scheme: &Scheme) {
            self.registered.push(text.clone());
        }
    }

    impl SchemeInstantiation for TemplateFixture {
        fn instantiate(&mut self, _scheme: &Scheme, _span: Span) -> Type {
            Self::instantiated()
        }
    }

    #[test]
    fn an_inline_template_registers_its_text() {
        let mut fixture = TemplateFixture::default();
        fixture.type_template();
        assert_eq!(fixture.registered, [Rc::from("1")]);
    }

    #[test]
    fn an_inline_template_has_its_instantiated_type() {
        let ty = TemplateFixture::default().type_template();
        assert_eq!(ty, Some(TemplateFixture::instantiated()));
    }
}

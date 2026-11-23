use super::{AnnotationLowering, ChildTyping, ExpectedChildTyping, LocalScope};
use crate::core::{Binder, Expr};
use crate::diagnostic::Diagnostic;
use crate::types::expectation::Expectation;
use crate::types::problems::MismatchContext;
use crate::types::ty::Type;
use crate::types::typed::{TypedBinder, TypedBodyItem};

pub(in crate::types) struct LocalBinding<'e> {
    binder: &'e Binder,
    value: &'e Expr,
}

impl<'e> LocalBinding<'e> {
    pub(in crate::types) fn new(binder: &'e Binder, value: &'e Expr) -> Self {
        Self { binder, value }
    }

    pub(in crate::types) fn bind<C>(
        &self,
        context: &mut C,
    ) -> Result<(TypedBodyItem, Type), Diagnostic>
    where
        C: ChildTyping + ExpectedChildTyping + AnnotationLowering + LocalScope,
    {
        let (value, ty) = match &self.binder.annotation {
            Some(annotation) => {
                let ty = context.lower_annotation(annotation)?;
                let annotation_context = MismatchContext::Annotation {
                    annotation: annotation.span(),
                };
                let expectation = Expectation::new(&ty, annotation_context);
                (context.type_child_against(self.value, expectation)?, ty)
            }
            None => {
                let value = context.type_child(self.value)?;
                let ty = value.ty.clone();
                (value, ty)
            }
        };
        context.bind_local(self.binder.binding, ty.clone());
        let binder = TypedBinder {
            binding: self.binder.binding,
            ty,
            span: self.binder.span,
            annotated: self.binder.annotation.is_some(),
            written_view: false,
        };
        Ok((TypedBodyItem::Let(binder, value), Type::unit()))
    }
}

#[cfg(test)]
mod tests {
    use super::LocalBinding;
    use crate::core::{Literal, PrimitiveType};
    use crate::types::forms::body_fixture::BodyFixture;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::ty::Type;
    use crate::types::typed::TypedBodyItem;

    struct LetFixture;

    impl LetFixture {
        fn bind(
            annotation: Option<PrimitiveType>,
            fixture: &mut BodyFixture,
        ) -> (TypedBodyItem, Type) {
            let binder = BodyFixture::binder(annotation);
            let value = FormFixtures::leaf(Literal::Bool(true));
            LocalBinding::new(&binder, &value)
                .bind(fixture)
                .unwrap_or_else(|_| panic!("the fixture types every value"))
        }
    }

    #[test]
    fn an_annotated_let_fits_its_value_to_the_annotation() {
        let mut fixture = BodyFixture::default();
        LetFixture::bind(Some(PrimitiveType::I64), &mut fixture);
        assert_eq!(fixture.fitted, [Type::Primitive(PrimitiveType::I64)]);
    }

    #[test]
    fn an_annotated_let_binds_the_annotation_type() {
        let mut fixture = BodyFixture::default();
        LetFixture::bind(Some(PrimitiveType::I64), &mut fixture);
        assert_eq!(
            fixture.bound,
            [(BodyFixture::LOCAL, Type::Primitive(PrimitiveType::I64))]
        );
    }

    #[test]
    fn an_unannotated_let_binds_its_value_type() {
        let mut fixture = BodyFixture::default();
        LetFixture::bind(None, &mut fixture);
        assert_eq!(fixture.bound, [(BodyFixture::LOCAL, Type::bool())]);
    }

    #[test]
    fn a_let_is_unit() {
        let (_, ty) = LetFixture::bind(None, &mut BodyFixture::default());
        assert_eq!(ty, Type::unit());
    }

    #[test]
    fn a_let_records_whether_it_was_annotated() {
        let (item, _) = LetFixture::bind(Some(PrimitiveType::I64), &mut BodyFixture::default());
        assert!(matches!(item, TypedBodyItem::Let(binder, _) if binder.annotated));
    }
}

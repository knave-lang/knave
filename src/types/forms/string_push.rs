use super::{StringPushTyping, Unknowns};
use crate::core::PrimitiveType;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::problems::TypeProblems;
use crate::types::synthetic_signature::SyntheticSignature;
use crate::types::synthetic_template::SyntheticTemplate;
use crate::types::ty::Type;
use crate::types::typed::{Borrow, TypedArgument, TypedExpr};

pub(in crate::types) struct StringPush {
    span: Span,
}

impl StringPush {
    const TARGET_POSITION: usize = 0;
    const ITEM_POSITION: usize = 1;

    pub(in crate::types) fn new(span: Span) -> Self {
        Self { span }
    }

    pub(in crate::types) fn targets<C: Unknowns>(context: &C, target: &Type) -> bool {
        context.resolve(target).without_reference() == Self::string()
    }

    pub(in crate::types) fn type_with<C: StringPushTyping>(
        &self,
        context: &mut C,
        target: TypedExpr,
        item: TypedExpr,
    ) -> Result<TypedExpr, Diagnostic> {
        let character = Type::Primitive(PrimitiveType::Char);
        let (template, parameter) = if context.resolve(&item.ty) == character {
            (SyntheticTemplate::PUSH.text(), character)
        } else {
            (SyntheticTemplate::PUSH_TEXT.text(), Type::str_ref())
        };
        let parameters = vec![Type::mutable(Self::string()), parameter];
        let target_borrow =
            context.pass_argument(Self::TARGET_POSITION, &parameters[0], &target)?;
        let pushed = Self::pass_item(context, &parameters[1], &item)?;
        let arguments = vec![
            TypedArgument {
                value: target,
                borrow: target_borrow,
            },
            TypedArgument {
                value: item,
                borrow: pushed,
            },
        ];
        let signature = SyntheticSignature {
            template,
            parameters,
            result: Type::unit(),
        };
        Ok(context.finish_call(signature, arguments, self.span))
    }

    fn pass_item<C: StringPushTyping>(
        context: &mut C,
        parameter: &Type,
        item: &TypedExpr,
    ) -> Result<Borrow, Diagnostic> {
        context
            .pass_argument(Self::ITEM_POSITION, parameter, item)
            .map_err(|_| Self::item_problem(context, item))
    }

    fn item_problem<C: StringPushTyping>(context: &mut C, item: &TypedExpr) -> Diagnostic {
        let found = context.resolve(&item.ty);
        if found == Self::string() {
            return TypeProblems::owned_string_push_item(item.span);
        }
        TypeProblems::string_push_item(&context.describe(&found), item.span)
    }

    fn string() -> Type {
        Type::Primitive(PrimitiveType::String)
    }
}

#[cfg(test)]
mod tests {
    use super::StringPush;
    use crate::core::{Literal, PrimitiveType};
    use crate::diagnostic::{Diagnostic, Help};
    use crate::source::Span;
    use crate::types::forms::form_fixtures::FormFixtures;
    use crate::types::forms::{SyntheticCalls, TypeDisplays, Unknowns};
    use crate::types::problems::TypeProblems;
    use crate::types::synthetic_call::SyntheticCall;
    use crate::types::synthetic_signature::SyntheticSignature;
    use crate::types::synthetic_template::SyntheticTemplate;
    use crate::types::ty::Type;
    use crate::types::typed::{Borrow, TypedArgument, TypedExpr};

    #[derive(Default)]
    struct StringFixture {
        refuses_item: bool,
        passed: Vec<usize>,
        calls: Vec<(String, Vec<Type>)>,
    }

    impl StringFixture {
        fn push(&mut self, item: Type) -> Result<TypedExpr, Diagnostic> {
            let target = FormFixtures::typed_leaf(Literal::Bool(true), StringPush::string());
            let item = FormFixtures::typed_leaf(Literal::Bool(true), item);
            StringPush::new(FormFixtures::span()).type_with(self, target, item)
        }

        fn character() -> Type {
            Type::Primitive(PrimitiveType::Char)
        }

        fn refusal_helps(&mut self, item: Type) -> Option<Vec<Help>> {
            self.refuses_item = true;
            Some(self.push(item).err()?.helps().to_vec())
        }
    }

    impl Unknowns for StringFixture {
        fn fresh(&mut self) -> Type {
            Type::bool()
        }

        fn resolve(&self, value: &Type) -> Type {
            value.clone()
        }

        fn is_open(&self, _value: &Type) -> bool {
            false
        }
    }

    impl SyntheticCalls for StringFixture {
        fn type_synthetic_call(
            &mut self,
            call: SyntheticCall<'_>,
        ) -> Result<TypedExpr, Diagnostic> {
            Ok(call.value)
        }

        fn pass_argument(
            &mut self,
            position: usize,
            _parameter: &Type,
            value: &TypedExpr,
        ) -> Result<Borrow, Diagnostic> {
            self.passed.push(position);
            if self.refuses_item && position == 1 {
                return Err(TypeProblems::wrong_argument_count(1, 0, value.span));
            }
            Ok(Borrow::None)
        }

        fn finish_call(
            &mut self,
            signature: SyntheticSignature<'_>,
            _arguments: Vec<TypedArgument>,
            _span: Span,
        ) -> TypedExpr {
            self.calls
                .push((signature.template.to_string(), signature.parameters));
            FormFixtures::typed_leaf(Literal::Bool(true), signature.result)
        }
    }

    impl TypeDisplays for StringFixture {
        fn show(&self, _ty: &Type) -> String {
            "shown".to_string()
        }

        fn describe(&mut self, _ty: &Type) -> String {
            "described".to_string()
        }
    }

    #[test]
    fn a_borrowed_string_is_a_string_target() {
        let target = Type::mutable(StringPush::string());
        assert!(StringPush::targets(&StringFixture::default(), &target));
    }

    #[test]
    fn a_vector_is_no_string_target() {
        assert!(!StringPush::targets(
            &StringFixture::default(),
            &Type::bool()
        ));
    }

    #[test]
    fn a_character_pushes_as_a_character() {
        let mut fixture = StringFixture::default();
        let _ = fixture.push(StringFixture::character());
        let expected = (
            SyntheticTemplate::PUSH.text().to_string(),
            vec![
                Type::mutable(StringPush::string()),
                StringFixture::character(),
            ],
        );
        assert_eq!(fixture.calls, [expected]);
    }

    #[test]
    fn text_pushes_as_a_string_reference() {
        let mut fixture = StringFixture::default();
        let _ = fixture.push(Type::str_ref());
        let expected = (
            SyntheticTemplate::PUSH_TEXT.text().to_string(),
            vec![Type::mutable(StringPush::string()), Type::str_ref()],
        );
        assert_eq!(fixture.calls, [expected]);
    }

    #[test]
    fn a_string_push_passes_its_target_then_its_item() {
        let mut fixture = StringFixture::default();
        let _ = fixture.push(Type::str_ref());
        assert_eq!(fixture.passed, [0, 1]);
    }

    #[test]
    fn an_owned_string_item_is_refused_as_owned() {
        let helps = StringFixture::default().refusal_helps(StringPush::string());
        let expected = TypeProblems::owned_string_push_item(FormFixtures::span());
        assert_eq!(helps, Some(expected.helps().to_vec()));
    }

    #[test]
    fn another_item_is_refused_as_no_text() {
        let helps = StringFixture::default().refusal_helps(Type::bool());
        let expected = TypeProblems::string_push_item("described", FormFixtures::span());
        assert_eq!(helps, Some(expected.helps().to_vec()));
    }
}

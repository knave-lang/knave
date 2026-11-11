use super::fragment::{Code, Precedence};
use super::type_context::TypeContext;
use crate::reader::{Number, NumberValue};
use crate::types::Type;

pub struct NumberLiteral<'n> {
    number: &'n Number,
    ty: &'n Type,
}

impl<'n> NumberLiteral<'n> {
    pub fn new(number: &'n Number, ty: &'n Type) -> Self {
        Self { number, ty }
    }

    pub fn code(&self, context: TypeContext) -> Code {
        let digits = self.digits();
        match (self.ty, context) {
            (Type::Primitive(primitive), TypeContext::Unknown) => {
                Self::signed(format!("{digits}_{}", primitive.name()))
            }
            _ => Self::signed(digits),
        }
    }

    pub fn digits(&self) -> String {
        let digits = self.number.text().trim_start_matches('+').to_string();
        match (self.number.value(), self.ty) {
            (NumberValue::Integer(_), Type::Primitive(primitive)) if primitive.is_float() => {
                format!("{digits}.0")
            }
            _ => digits,
        }
    }

    fn signed(text: String) -> Code {
        let precedence = if text.starts_with('-') {
            Precedence::Prefix
        } else {
            Precedence::Atom
        };
        Code::with(text, precedence)
    }
}

#[cfg(test)]
mod tests {
    use super::NumberLiteral;
    use crate::core::PrimitiveType;
    use crate::emit::fragment::Precedence;
    use crate::emit::type_context::TypeContext;
    use crate::reader::{Number, NumberValue};
    use crate::types::{GenericId, Type};

    struct NumberFixture;

    impl NumberFixture {
        fn integer(text: &str) -> Number {
            let value = text.parse().expect("the fixture writes an integer");
            Number::new(text.to_string(), NumberValue::Integer(value))
        }

        fn primitive(primitive: PrimitiveType) -> Type {
            Type::Primitive(primitive)
        }
    }

    #[test]
    fn writes_the_digits_without_a_plus_sign() {
        let number = NumberFixture::integer("+7");
        let ty = NumberFixture::primitive(PrimitiveType::I64);
        assert_eq!(
            NumberLiteral::new(&number, &ty)
                .code(TypeContext::Known)
                .text(),
            "7"
        );
    }

    #[test]
    fn writes_an_integer_as_a_float_where_a_float_is_expected() {
        let number = NumberFixture::integer("7");
        let ty = NumberFixture::primitive(PrimitiveType::F64);
        assert_eq!(NumberLiteral::new(&number, &ty).digits(), "7.0");
    }

    #[test]
    fn suffixes_the_primitive_type() {
        let number = NumberFixture::integer("7");
        let ty = NumberFixture::primitive(PrimitiveType::U8);
        assert_eq!(
            NumberLiteral::new(&number, &ty)
                .code(TypeContext::Unknown)
                .text(),
            "7_u8"
        );
    }

    #[test]
    fn leaves_a_generic_number_unsuffixed() {
        let number = NumberFixture::integer("7");
        let ty = Type::Generic(GenericId::new(0));
        assert_eq!(
            NumberLiteral::new(&number, &ty)
                .code(TypeContext::Unknown)
                .text(),
            "7"
        );
    }

    #[test]
    fn a_negative_number_is_a_prefix_expression() {
        let number = NumberFixture::integer("-7");
        let ty = NumberFixture::primitive(PrimitiveType::I64);
        let code = NumberLiteral::new(&number, &ty).code(TypeContext::Unknown);
        assert_eq!(code.precedence, Precedence::Prefix);
    }
}

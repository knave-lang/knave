use super::fragment::{Code, Precedence};
use crate::core::Conversion;

pub struct ConvertedArgument {
    conversion: Conversion,
}

impl ConvertedArgument {
    const INDEX_CALL: (&'static str, &'static str) = ("crate::rt::index(", ")");
    const OPTIONAL_INDEX_CALL: (&'static str, &'static str) =
        ("usize::try_from(", ").unwrap_or(usize::MAX)");

    pub fn new(conversion: Conversion) -> Self {
        Self { conversion }
    }

    pub fn code(&self, argument: &Code) -> Code {
        if let Some(literal) = Self::unsigned_literal(argument) {
            return literal;
        }
        let (open, close) = self.call();
        Code::joined(
            vec![Code::atom(open), argument.clone(), Code::atom(close)],
            Precedence::Atom,
        )
    }

    fn call(&self) -> (&'static str, &'static str) {
        match self.conversion {
            Conversion::Index => Self::INDEX_CALL,
            Conversion::OptionalIndex => Self::OPTIONAL_INDEX_CALL,
        }
    }

    fn unsigned_literal(argument: &Code) -> Option<Code> {
        let text = argument.text();
        let suffix = format!("_{}", Conversion::ARGUMENT.names().rust());
        let digits = text.strip_suffix(&suffix).unwrap_or(&text);
        let unsigned = digits.starts_with(|character: char| character.is_ascii_digit())
            && digits
                .chars()
                .all(|character| character.is_ascii_digit() || character == '_');
        (argument.is_atomic() && unsigned).then(|| Code::atom(digits))
    }
}

#[cfg(test)]
mod tests {
    use super::ConvertedArgument;
    use crate::core::Conversion;
    use crate::emit::fragment::{Code, Precedence};

    struct ConversionFixture;

    impl ConversionFixture {
        fn converted(conversion: Conversion, argument: Code) -> String {
            ConvertedArgument::new(conversion).code(&argument).text()
        }
    }

    #[test]
    fn writes_an_unsigned_literal_bare() {
        for conversion in [Conversion::Index, Conversion::OptionalIndex] {
            assert_eq!(
                ConversionFixture::converted(conversion, Code::atom("1_000")),
                "1_000"
            );
        }
    }

    #[test]
    fn checks_any_other_index_at_run_time() {
        assert_eq!(
            ConversionFixture::converted(Conversion::Index, Code::atom("i")),
            "crate::rt::index(i)"
        );
    }

    #[test]
    fn maps_any_other_optional_index_past_the_end() {
        assert_eq!(
            ConversionFixture::converted(
                Conversion::OptionalIndex,
                Code::with("-1_i64", Precedence::Prefix)
            ),
            "usize::try_from(-1_i64).unwrap_or(usize::MAX)"
        );
    }

    #[test]
    fn writes_a_suffixed_unsigned_literal_bare() {
        assert_eq!(
            ConversionFixture::converted(Conversion::OptionalIndex, Code::atom("4_i64")),
            "4"
        );
    }

    #[test]
    fn converts_a_literal_of_another_type() {
        assert_eq!(
            ConversionFixture::converted(Conversion::Index, Code::atom("4_u8")),
            "crate::rt::index(4_u8)"
        );
    }

    #[test]
    fn converts_digits_that_are_not_atomic() {
        assert_eq!(
            ConversionFixture::converted(Conversion::Index, Code::with("1", Precedence::Operator)),
            "crate::rt::index(1)"
        );
    }

    #[test]
    fn converts_a_name_that_starts_like_a_number() {
        assert_eq!(
            ConversionFixture::converted(Conversion::Index, Code::atom("_1")),
            "crate::rt::index(_1)"
        );
    }

    #[test]
    fn converts_an_operation_without_parentheses() {
        assert_eq!(
            ConversionFixture::converted(
                Conversion::Index,
                Code::with("i + 1", Precedence::Operator)
            ),
            "crate::rt::index(i + 1)"
        );
    }
}

use super::representation::Representation;
use super::scheme_position::SchemePosition;
use super::scheme_positions::SchemePositions;
use super::text_value::TextValue;
use super::transfer::Transfer;
use super::type_path::TypePath;
use crate::types::Type;

pub struct TextConversion;

impl TextConversion {
    pub fn is_conversion(parameters: &[Type], result: &Type) -> bool {
        match parameters {
            [parameter] => {
                Self::is_top(parameter, Representation::Lent)
                    && Self::is_top(result, Representation::View)
            }
            _ => false,
        }
    }

    pub fn converted(argument: &TextValue) -> TextValue {
        match argument.place() {
            Some(_) => argument.clone().transferred(Transfer::Refer),
            None => argument.clone(),
        }
    }

    fn is_top(declared: &Type, expected: Representation) -> bool {
        SchemePositions::of(declared, declared)
            == [SchemePosition::Written {
                path: TypePath::default(),
                written: expected,
            }]
    }
}

#[cfg(test)]
mod tests {
    use super::TextConversion;
    use crate::core::PrimitiveType;
    use crate::types::Type;

    struct Signatures;

    impl Signatures {
        fn lent_text() -> Type {
            Type::shared(Type::Primitive(PrimitiveType::String))
        }
    }

    #[test]
    fn a_view_of_lent_text_is_a_conversion() {
        assert!(TextConversion::is_conversion(
            &[Signatures::lent_text()],
            &Type::str_ref()
        ));
    }

    #[test]
    fn a_view_of_a_view_is_no_conversion() {
        assert!(!TextConversion::is_conversion(
            &[Type::str_ref()],
            &Type::str_ref()
        ));
    }

    #[test]
    fn a_template_of_two_parameters_is_no_conversion() {
        let parameters = [Signatures::lent_text(), Signatures::lent_text()];
        assert!(!TextConversion::is_conversion(
            &parameters,
            &Type::str_ref()
        ));
    }
}

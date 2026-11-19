use super::access::Access;
use super::representation::Representation;
use super::type_path::TypePath;
use crate::core::PrimitiveType;
use crate::types::Type;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextPosition {
    pub path: TypePath,
    pub written: Representation,
}

impl TextPosition {
    pub fn is_written_reference(&self) -> bool {
        matches!(self.written, Representation::View | Representation::Lent)
    }
}

pub struct TextPositions {
    found: Vec<TextPosition>,
}

impl TextPositions {
    pub fn of(ty: &Type) -> Vec<TextPosition> {
        let mut positions = Self { found: Vec::new() };
        positions.visit(ty, TypePath::default(), Access::Direct);
        positions.found
    }

    pub fn holds_text(ty: &Type) -> bool {
        !Self::of(ty).is_empty()
    }

    fn visit(&mut self, ty: &Type, path: TypePath, access: Access) {
        match ty {
            Type::Primitive(primitive) => self.visit_primitive(*primitive, path, access),
            Type::Ref(_, inner) | Type::Mut(_, inner) => {
                self.visit(inner, path, Access::Referenced)
            }
            Type::Named(_, arguments) => {
                arguments.iter().enumerate().for_each(|(part, argument)| {
                    self.visit(argument, path.child(part), Access::Direct)
                })
            }
            Type::Var(_) | Type::Generic(_) | Type::Function(..) | Type::TraitObject(_) => {}
        }
    }

    fn visit_primitive(&mut self, primitive: PrimitiveType, path: TypePath, access: Access) {
        if let Some(written) = Representation::written(primitive, access) {
            self.found.push(TextPosition { path, written });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Representation, TextPositions, TypePath};
    use crate::core::PrimitiveType;
    use crate::types::Type;

    struct Types;

    impl Types {
        fn text(primitive: PrimitiveType) -> Type {
            Type::Primitive(primitive)
        }
    }

    #[test]
    fn a_shared_string_at_the_top_is_lent_text() {
        let positions = TextPositions::of(&Type::shared(Types::text(PrimitiveType::String)));
        assert_eq!(positions[0].written, Representation::Lent);
        assert!(positions[0].path.is_top());
    }

    #[test]
    fn a_string_at_the_top_is_owned_text() {
        let positions = TextPositions::of(&Types::text(PrimitiveType::String));
        assert_eq!(positions[0].written, Representation::Owned);
    }

    #[test]
    fn a_reference_to_a_string_inside_a_type_is_lent() {
        let option = Type::Named(
            crate::core::BindingId::from_index(0),
            vec![Type::shared(Types::text(PrimitiveType::String))],
        );
        let positions = TextPositions::of(&option);
        assert_eq!(positions[0].written, Representation::Lent);
        assert_eq!(positions[0].path, TypePath::default().child(0));
    }

    #[test]
    fn views_and_lent_text_are_written_references() {
        let lent = TextPositions::of(&Type::shared(Types::text(PrimitiveType::String)));
        let view = TextPositions::of(&Type::str_ref());
        let owned = TextPositions::of(&Types::text(PrimitiveType::String));
        assert!(lent[0].is_written_reference());
        assert!(view[0].is_written_reference());
        assert!(!owned[0].is_written_reference());
    }

    #[test]
    fn unsized_text_inside_a_type_is_owned() {
        let boxed = Type::Named(
            crate::core::BindingId::from_index(0),
            vec![Types::text(PrimitiveType::Str)],
        );
        assert_eq!(TextPositions::of(&boxed)[0].written, Representation::Owned);
    }

    #[test]
    fn a_function_type_holds_no_text_position() {
        let function = Type::Function(
            crate::core::ClosureKind::Fn,
            vec![Type::str_ref()],
            Box::new(Type::unit()),
        );
        assert!(!TextPositions::holds_text(&function));
    }
}

use super::access::Access;
use super::representation::Representation;
use super::scheme_position::SchemePosition;
use super::text_position::TextPositions;
use super::type_path::TypePath;
use crate::core::PrimitiveType;
use crate::types::{GenericId, Type};

#[derive(Clone)]
struct Place {
    path: TypePath,
    access: Access,
}

impl Place {
    fn referenced(&self) -> Self {
        Self {
            path: self.path.clone(),
            access: Access::Referenced,
        }
    }

    fn child(&self, part: usize) -> Self {
        Self {
            path: self.path.child(part),
            access: Access::Direct,
        }
    }
}

pub struct SchemePositions {
    found: Vec<SchemePosition>,
}

impl SchemePositions {
    pub fn of(declared: &Type, instantiated: &Type) -> Vec<SchemePosition> {
        let mut positions = Self { found: Vec::new() };
        let top = Place {
            path: TypePath::default(),
            access: Access::Direct,
        };
        positions.visit(declared, instantiated, top);
        positions.found
    }

    fn visit(&mut self, declared: &Type, instantiated: &Type, place: Place) {
        match (declared, instantiated) {
            (Type::Generic(generic), _) => self.visit_generic(*generic, instantiated, place),
            (Type::Ref(_, inner) | Type::Mut(_, inner), _) => {
                self.visit(inner, Self::referent(instantiated), place.referenced())
            }
            (_, Type::Ref(_, inner) | Type::Mut(_, inner)) => self.visit(declared, inner, place),
            (Type::Primitive(primitive), _) => self.visit_written(*primitive, place),
            (Type::Named(_, declared_parts), Type::Named(_, parts)) => {
                for (part, (declared, instantiated)) in declared_parts.iter().zip(parts).enumerate()
                {
                    self.visit(declared, instantiated, place.child(part));
                }
            }
            _ => {}
        }
    }

    fn referent(ty: &Type) -> &Type {
        match ty {
            Type::Ref(_, inner) | Type::Mut(_, inner) => inner,
            other => other,
        }
    }

    fn visit_generic(&mut self, generic: GenericId, instantiated: &Type, place: Place) {
        for position in TextPositions::of(instantiated) {
            self.found.push(SchemePosition::Generic {
                generic,
                outer: place.path.clone(),
                inner: position.path,
                access: place.access,
            });
        }
    }

    fn visit_written(&mut self, primitive: PrimitiveType, place: Place) {
        if let Some(written) = Representation::written(primitive, place.access) {
            self.found.push(SchemePosition::Written {
                path: place.path,
                written,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Access, SchemePosition, SchemePositions, TypePath};
    use crate::core::{BindingId, PrimitiveType};
    use crate::ownership::text::representation::Representation;
    use crate::types::{GenericId, Type};

    struct Types;

    impl Types {
        fn vec_of(item: Type) -> Type {
            Type::Named(BindingId::from_index(7), vec![item])
        }

        fn text() -> Type {
            Type::Primitive(PrimitiveType::String)
        }
    }

    #[test]
    fn a_generic_element_matches_the_text_it_was_given() {
        let generic = GenericId::new(0);
        let positions = SchemePositions::of(
            &Type::mutable(Types::vec_of(Type::Generic(generic))),
            &Types::vec_of(Types::text()),
        );
        assert_eq!(
            positions,
            vec![SchemePosition::Generic {
                generic,
                outer: TypePath::default().child(0),
                inner: TypePath::default(),
                access: Access::Direct,
            }]
        );
    }

    #[test]
    fn a_generic_behind_a_shared_reference_is_referenced() {
        let generic = GenericId::new(0);
        let positions = SchemePositions::of(
            &Type::shared(Type::Generic(generic)),
            &Type::shared(Types::text()),
        );
        assert!(matches!(
            positions[0],
            SchemePosition::Generic {
                access: Access::Referenced,
                ..
            }
        ));
        assert!(positions[0].path().is_top());
    }

    #[test]
    fn a_written_string_behind_a_reference_is_lent() {
        let positions = SchemePositions::of(&Type::shared(Types::text()), &Types::text());
        assert_eq!(
            positions,
            vec![SchemePosition::Written {
                path: TypePath::default(),
                written: Representation::Lent,
            }]
        );
    }

    #[test]
    fn a_written_string_is_owned() {
        let positions = SchemePositions::of(&Types::text(), &Type::shared(Types::text()));
        assert!(matches!(
            positions[0],
            SchemePosition::Written {
                written: Representation::Owned,
                ..
            }
        ));
    }

    #[test]
    fn a_generic_path_runs_through_its_outer_and_inner_parts() {
        let generic = GenericId::new(0);
        let pair = Type::Named(BindingId::from_index(8), vec![Types::text()]);
        let positions =
            SchemePositions::of(&Types::vec_of(Type::Generic(generic)), &Types::vec_of(pair));
        assert_eq!(positions[0].path(), TypePath::default().child(0).child(0));
    }

    #[test]
    fn a_written_path_is_its_own_place() {
        let positions =
            SchemePositions::of(&Types::vec_of(Types::text()), &Types::vec_of(Types::text()));
        assert_eq!(positions[0].path(), TypePath::default().child(0));
    }

    #[test]
    fn a_reference_declared_over_a_value_still_reaches_its_generic() {
        let generic = GenericId::new(0);
        let positions = SchemePositions::of(&Type::shared(Type::Generic(generic)), &Types::text());
        assert!(matches!(
            positions.as_slice(),
            [SchemePosition::Generic {
                access: Access::Referenced,
                ..
            }]
        ));
    }
}

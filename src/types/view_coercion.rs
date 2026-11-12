use super::rust_types::RustTypeTable;
use super::ty::Type;
use crate::core::PrimitiveType;

#[derive(Clone, Copy)]
pub struct ViewCoercion<'a> {
    view: &'a Type,
    owned: &'a Type,
}

impl<'a> ViewCoercion<'a> {
    pub fn new(view: &'a Type, owned: &'a Type) -> Self {
        Self { view, owned }
    }

    pub fn views_string(&self) -> bool {
        matches!(
            (self.view, self.owned),
            (
                Type::Primitive(PrimitiveType::Str),
                Type::Primitive(PrimitiveType::String)
            )
        )
    }

    pub fn viewed_items(&self, rust_types: &RustTypeTable) -> Option<Vec<(Type, Type)>> {
        match (self.view, self.owned) {
            (Type::Named(slice, items), Type::Named(vector, elements))
                if rust_types.is_slice(*slice) && rust_types.is_vector(*vector) =>
            {
                Some(
                    items
                        .iter()
                        .cloned()
                        .zip(elements.iter().cloned())
                        .collect(),
                )
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ViewCoercion;
    use crate::core::PrimitiveType;
    use crate::types::{Type, TypedProgram, TypedSource};

    struct ViewFixture {
        program: TypedProgram,
    }

    impl ViewFixture {
        const SOURCE: &str = "
            (defn readings ((items (vec i64))) items)
            (defn window ((items (ref (slice u8)))) (len items))
            (defn reading ((value (option i64))) value)
            (defn main () (println \"Leaf\"))";

        fn new() -> Self {
            Self {
                program: TypedSource::expand(Self::SOURCE).typed(),
            }
        }

        fn viewed_items(&self, view: &Type, owned: &Type) -> Option<Vec<(Type, Type)>> {
            ViewCoercion::new(view, owned)
                .viewed_items(self.program.declarations.types.rust_types())
        }

        fn parameter(&self, function: &str) -> Type {
            let definition = self
                .program
                .modules
                .iter()
                .flat_map(|module| &module.definitions)
                .find(|definition| self.program.bindings.get(definition.binding).name() == function)
                .expect("the source defines the function");
            let Type::Function(_, parameters, _) = &definition.scheme.body else {
                panic!("a defn has a function type");
            };
            parameters[0].referent().clone()
        }
    }

    #[test]
    fn a_slice_relates_its_items_to_the_elements_of_a_vector() {
        let fixture = ViewFixture::new();
        let items =
            fixture.viewed_items(&fixture.parameter("window"), &fixture.parameter("readings"));
        assert_eq!(
            items,
            Some(vec![(
                Type::Primitive(PrimitiveType::U8),
                Type::Primitive(PrimitiveType::I64)
            )])
        );
    }

    #[test]
    fn a_type_that_is_not_a_slice_views_no_vector() {
        let fixture = ViewFixture::new();
        assert!(
            fixture
                .viewed_items(
                    &fixture.parameter("reading"),
                    &fixture.parameter("readings")
                )
                .is_none()
        );
    }

    #[test]
    fn a_slice_views_no_type_that_is_not_a_vector() {
        let fixture = ViewFixture::new();
        assert!(
            fixture
                .viewed_items(&fixture.parameter("window"), &fixture.parameter("reading"))
                .is_none()
        );
    }

    #[test]
    fn a_str_views_a_string() {
        let view = Type::Primitive(PrimitiveType::Str);
        let owned = Type::Primitive(PrimitiveType::String);
        assert!(ViewCoercion::new(&view, &owned).views_string());
    }

    #[test]
    fn a_string_does_not_view_a_str() {
        let view = Type::Primitive(PrimitiveType::String);
        let owned = Type::Primitive(PrimitiveType::Str);
        assert!(!ViewCoercion::new(&view, &owned).views_string());
    }
}

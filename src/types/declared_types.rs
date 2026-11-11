use super::ty::Type;
use super::type_declarations::TypeDeclarations;
use super::view_coercion::ViewCoercion;
use crate::core::BindingId;

#[derive(Clone, Copy)]
pub(super) struct DeclaredTypes<'a> {
    types: &'a TypeDeclarations,
}

impl<'a> DeclaredTypes<'a> {
    pub(super) fn new(types: &'a TypeDeclarations) -> Self {
        Self { types }
    }

    pub(super) fn field_type(
        &self,
        owner: BindingId,
        label: &str,
        arguments: &[Type],
    ) -> Option<Type> {
        self.types
            .structs
            .get(owner)
            .and_then(|definition| definition.instantiated_field(label, arguments))
    }

    pub(super) fn declares(&self, binding: BindingId) -> bool {
        self.is_struct(binding) || self.types.enums.get(binding).is_some()
    }

    pub(super) fn is_struct(&self, binding: BindingId) -> bool {
        self.types.structs.get(binding).is_some()
    }

    pub(super) fn is_box(&self, candidate: &Type) -> bool {
        match candidate {
            Type::Named(binding, arguments) => {
                Some(*binding) == self.types.box_binding && arguments.len() == 1
            }
            _ => false,
        }
    }

    pub(super) fn is_unsized(&self, candidate: &Type) -> bool {
        match candidate {
            Type::Named(binding, _) => self.types.rust_types.is_slice(*binding),
            _ => candidate.is_unsized(),
        }
    }

    pub(super) fn viewed_items(&self, coercion: ViewCoercion) -> Option<Vec<(Type, Type)>> {
        coercion.viewed_items(&self.types.rust_types)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::PrimitiveType;
    use crate::types::Type;
    use crate::types::declarations_fixture::DeclarationsFixture;
    use crate::types::view_coercion::ViewCoercion;

    const SOURCE: &str = "
        (struct reading (value i64))
        (defn readings ((items (vec i64))) items)
        (defn window ((items (ref (slice u8)))) (len items))
        (defn main () (println \"Leaf\"))";

    #[test]
    fn a_declared_field_has_its_declared_type() {
        let fixture = DeclarationsFixture::new(SOURCE);
        let reading = fixture.type_named("reading");
        assert_eq!(
            fixture.declared_types().field_type(reading, "value", &[]),
            Some(Type::Primitive(PrimitiveType::I64))
        );
    }

    #[test]
    fn an_undeclared_field_has_no_type() {
        let fixture = DeclarationsFixture::new(SOURCE);
        let reading = fixture.type_named("reading");
        assert_eq!(
            fixture.declared_types().field_type(reading, "missing", &[]),
            None
        );
    }

    #[test]
    fn a_slice_relates_its_items_to_the_elements_of_a_vector() {
        let fixture = DeclarationsFixture::new(SOURCE);
        let slice = fixture.parameter("window").referent().clone();
        let vector = fixture.parameter("readings").referent().clone();
        assert_eq!(
            fixture
                .declared_types()
                .viewed_items(ViewCoercion::new(&slice, &vector)),
            Some(vec![(
                Type::Primitive(PrimitiveType::U8),
                Type::Primitive(PrimitiveType::I64)
            )])
        );
    }

    #[test]
    fn a_string_slice_is_unsized() {
        let fixture = DeclarationsFixture::new(SOURCE);
        assert!(
            fixture
                .declared_types()
                .is_unsized(&Type::Primitive(PrimitiveType::Str))
        );
    }

    #[test]
    fn a_slice_is_unsized() {
        let fixture = DeclarationsFixture::new(SOURCE);
        let slice = fixture.parameter("window").referent().clone();
        assert!(fixture.declared_types().is_unsized(&slice));
    }

    #[test]
    fn a_vector_is_sized() {
        let fixture = DeclarationsFixture::new(SOURCE);
        let vector = fixture.parameter("readings").referent().clone();
        assert!(!fixture.declared_types().is_unsized(&vector));
    }

    #[test]
    fn a_number_is_sized() {
        let fixture = DeclarationsFixture::new(SOURCE);
        assert!(
            !fixture
                .declared_types()
                .is_unsized(&Type::Primitive(PrimitiveType::I64))
        );
    }
}

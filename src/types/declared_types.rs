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
    use super::DeclaredTypes;
    use crate::core::{BindingId, PrimitiveType};
    use crate::types::type_declarations::TypeDeclarations;
    use crate::types::view_coercion::ViewCoercion;
    use crate::types::{Type, TypedProgram, TypedSource};

    struct DeclarationsFixture {
        program: TypedProgram,
        types: TypeDeclarations,
    }

    impl DeclarationsFixture {
        const SOURCE: &str = "
            (struct reading (value i64))
            (defn readings ((items (vec i64))) items)
            (defn window ((items (ref (slice u8)))) (len items))
            (defn main () (println \"Leaf\"))";

        fn new() -> Self {
            let mut program = TypedSource::expand(Self::SOURCE).typed();
            let types = TypeDeclarations {
                structs: std::mem::take(&mut program.structs),
                rust_types: std::mem::take(&mut program.rust_types),
                ..TypeDeclarations::default()
            };
            Self { program, types }
        }

        fn declarations(&self) -> DeclaredTypes<'_> {
            DeclaredTypes::new(&self.types)
        }

        fn reading(&self) -> BindingId {
            self.types
                .structs
                .iter()
                .map(|(binding, _)| binding)
                .find(|binding| self.program.bindings.get(*binding).name() == "reading")
                .expect("the source declares the struct")
        }

        fn parameter_referent(&self, function: &str) -> Type {
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
    fn a_declared_field_has_its_declared_type() {
        let fixture = DeclarationsFixture::new();
        let reading = fixture.reading();
        assert_eq!(
            fixture.declarations().field_type(reading, "value", &[]),
            Some(Type::Primitive(PrimitiveType::I64))
        );
    }

    #[test]
    fn an_undeclared_field_has_no_type() {
        let fixture = DeclarationsFixture::new();
        let reading = fixture.reading();
        assert_eq!(
            fixture.declarations().field_type(reading, "missing", &[]),
            None
        );
    }

    #[test]
    fn a_slice_relates_its_items_to_the_elements_of_a_vector() {
        let fixture = DeclarationsFixture::new();
        let slice = fixture.parameter_referent("window");
        let vector = fixture.parameter_referent("readings");
        assert_eq!(
            fixture
                .declarations()
                .viewed_items(ViewCoercion::new(&slice, &vector)),
            Some(vec![(
                Type::Primitive(PrimitiveType::U8),
                Type::Primitive(PrimitiveType::I64)
            )])
        );
    }

    #[test]
    fn a_string_slice_is_unsized() {
        let fixture = DeclarationsFixture::new();
        assert!(
            fixture
                .declarations()
                .is_unsized(&Type::Primitive(PrimitiveType::Str))
        );
    }

    #[test]
    fn a_slice_is_unsized() {
        let fixture = DeclarationsFixture::new();
        let slice = fixture.parameter_referent("window");
        assert!(fixture.declarations().is_unsized(&slice));
    }

    #[test]
    fn a_vector_is_sized() {
        let fixture = DeclarationsFixture::new();
        let vector = fixture.parameter_referent("readings");
        assert!(!fixture.declarations().is_unsized(&vector));
    }

    #[test]
    fn a_number_is_sized() {
        let fixture = DeclarationsFixture::new();
        assert!(
            !fixture
                .declarations()
                .is_unsized(&Type::Primitive(PrimitiveType::I64))
        );
    }
}

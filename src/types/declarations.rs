use super::rust_types::RustTypeTable;
use super::structs::StructTable;
use super::ty::Type;
use super::view_coercion::ViewCoercion;
use crate::core::BindingId;

#[derive(Clone, Copy)]
pub(super) struct Declarations<'a> {
    structs: &'a StructTable,
    rust_types: &'a RustTypeTable,
}

impl<'a> Declarations<'a> {
    pub(super) fn new(structs: &'a StructTable, rust_types: &'a RustTypeTable) -> Self {
        Self {
            structs,
            rust_types,
        }
    }

    pub(super) fn field_type(
        &self,
        owner: BindingId,
        label: &str,
        arguments: &[Type],
    ) -> Option<Type> {
        self.structs
            .get(owner)
            .and_then(|definition| definition.instantiated_field(label, arguments))
    }

    pub(super) fn viewed_items(&self, coercion: ViewCoercion) -> Option<Vec<(Type, Type)>> {
        coercion.viewed_items(self.rust_types)
    }
}

#[cfg(test)]
mod tests {
    use super::Declarations;
    use crate::core::{BindingId, PrimitiveType};
    use crate::types::view_coercion::ViewCoercion;
    use crate::types::{Type, TypedProgram, TypedSource};

    struct DeclarationsFixture {
        program: TypedProgram,
    }

    impl DeclarationsFixture {
        const SOURCE: &str = "
            (struct reading (value i64))
            (defn readings ((items (vec i64))) items)
            (defn window ((items (ref (slice u8)))) (len items))
            (defn main () (println \"Leaf\"))";

        fn new() -> Self {
            Self {
                program: TypedSource::expand(Self::SOURCE).typed(),
            }
        }

        fn declarations(&self) -> Declarations<'_> {
            Declarations::new(&self.program.structs, &self.program.rust_types)
        }

        fn reading(&self) -> BindingId {
            self.program
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
}

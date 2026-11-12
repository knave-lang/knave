use super::unsized_parameters::UnsizedParameters;
use crate::core::BindingId;
use crate::types::{
    BoxedPayload, EnumTable, GenericId, RustTypeTable, StructTable, TypeDeclarations,
};

#[derive(Clone, Copy)]
pub struct EmittedTypes<'a> {
    declared: &'a TypeDeclarations,
    unsized_parameters: &'a UnsizedParameters,
}

impl<'a> EmittedTypes<'a> {
    pub fn new(declared: &'a TypeDeclarations, unsized_parameters: &'a UnsizedParameters) -> Self {
        Self {
            declared,
            unsized_parameters,
        }
    }

    pub fn structs(&self) -> &'a StructTable {
        self.declared.structs()
    }

    pub fn enums(&self) -> &'a EnumTable {
        self.declared.enums()
    }

    pub fn rust_types(&self) -> &'a RustTypeTable {
        self.declared.rust_types()
    }

    pub fn box_binding(&self) -> Option<BindingId> {
        self.declared.box_binding()
    }

    pub fn boxes(&self, payload: &BoxedPayload) -> bool {
        self.declared.boxes(payload)
    }

    pub fn is_unsized(&self, generic: GenericId) -> bool {
        self.unsized_parameters.contains(generic)
    }
}

#[cfg(test)]
mod tests {
    use super::EmittedTypes;
    use crate::emit::plan::EmitPlan;
    use crate::emit::unsized_parameters::UnsizedParameters;
    use crate::ownership::OwnershipChecker;
    use crate::source::{SourceId, Span};
    use crate::types::{GenericId, Type, TypedProgram, TypedSource};

    struct MeasureFixture {
        program: TypedProgram,
    }

    impl MeasureFixture {
        const MEASURE: &str = "
            (def first-slice
              (rust \"{0}.as_slice()\"
                (where (a) (fn ((ref (vec a))) (ref (slice a))))))
            (defn measure (value) (len value))";
        const SLICE_CALL: &str = "
            (defn main ()
              (local numbers (vec 1 2 3))
              (println (measure (first-slice numbers))))";
        const VECTOR_CALL: &str = "
            (defn main ()
              (local numbers (vec 1 2 3))
              (println (measure (ref numbers))))";

        fn calling(main: &str) -> Self {
            let source = format!("{}\n{main}", Self::MEASURE);
            let mut program = TypedSource::expand(&source).typed();
            OwnershipChecker::check(&mut program).expect("the fixture checks");
            Self { program }
        }

        fn measured_generic(&self) -> GenericId {
            let definition = self
                .program
                .modules
                .iter()
                .flat_map(|module| &module.definitions)
                .find(|definition| {
                    self.program.bindings.get(definition.binding).name() == "measure"
                })
                .expect("the fixture defines measure");
            let Type::Function(_, parameters, _) = &definition.scheme.body else {
                panic!("measure is a function");
            };
            match parameters[0].referent() {
                Type::Generic(generic) => *generic,
                other => panic!("measure takes a generic, not {other:?}"),
            }
        }

        fn measured_generic_is_unsized(&self) -> bool {
            let plan = EmitPlan::build(&self.program, Span::new(SourceId::new(0), 0..0))
                .unwrap_or_else(|_| panic!("the fixture plans"));
            let unsized_parameters = UnsizedParameters::of(&self.program, &plan);
            EmittedTypes::new(&self.program.declarations.types(), &unsized_parameters)
                .is_unsized(self.measured_generic())
        }
    }

    #[test]
    fn a_generic_instantiated_with_a_slice_is_unsized() {
        let fixture = MeasureFixture::calling(MeasureFixture::SLICE_CALL);
        assert!(fixture.measured_generic_is_unsized());
    }

    #[test]
    fn a_generic_instantiated_with_a_vector_reference_stays_sized() {
        let fixture = MeasureFixture::calling(MeasureFixture::VECTOR_CALL);
        assert!(!fixture.measured_generic_is_unsized());
    }
}

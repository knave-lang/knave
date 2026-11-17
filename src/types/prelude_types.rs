use super::pipelines::Pipelines;
use super::rust_types::{RustType, RustTypeTable};
use super::ty::Type;

pub(super) struct PreludeTypes<'t> {
    rust_types: &'t RustTypeTable,
}

impl<'t> PreludeTypes<'t> {
    pub(super) fn new(rust_types: &'t RustTypeTable) -> Self {
        Self { rust_types }
    }

    pub(super) fn is_pipeline(&self, ty: &Type) -> bool {
        Pipelines::new(self.rust_types).is_pipeline(ty)
    }

    pub(super) fn slice_of(&self, element: Type) -> Type {
        let slice = self
            .rust_types
            .binding_with_template(RustType::SLICE_TEMPLATE)
            .expect("the prelude defines a slice type");
        Type::Named(slice, vec![element])
    }

    pub(super) fn option_type(&self, value: Type) -> Type {
        let option = self
            .rust_types
            .option_binding()
            .expect("the prelude defines an option type");
        Type::Named(option, vec![Type::shared(value)])
    }

    pub(super) fn iterator_type(&self, item: Type) -> Type {
        let iterator = self
            .rust_types
            .iterator_binding()
            .expect("the prelude defines an iterator type");
        Type::Named(iterator, vec![item])
    }

    pub(super) fn pair_of(&self, first: Type, second: Type) -> Type {
        let pair = self
            .rust_types
            .pair_binding()
            .expect("the prelude defines a pair type");
        Type::Named(pair, vec![first, second])
    }

    pub(super) fn nested_vector_of(&self, depth: usize, element: &Type) -> Type {
        let vector = self
            .rust_types
            .binding_with_template(RustType::VECTOR_TEMPLATE)
            .expect("the prelude defines a vector type");
        (0..depth).fold(element.clone(), |ty, _| Type::Named(vector, vec![ty]))
    }
}

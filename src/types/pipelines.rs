use super::rust_types::RustTypeTable;
use super::ty::Type;

#[derive(Clone, Copy)]
pub struct Pipelines<'a> {
    rust_types: &'a RustTypeTable,
}

impl<'a> Pipelines<'a> {
    pub fn new(rust_types: &'a RustTypeTable) -> Self {
        Self { rust_types }
    }

    pub fn is_pipeline(&self, ty: &Type) -> bool {
        matches!(ty, Type::Named(binding, _) if self.rust_types.is_iterator(*binding))
    }

    pub fn holds_an_iterator(&self, ty: &Type) -> bool {
        match ty {
            Type::Named(binding, arguments) => {
                self.rust_types.is_iterator(*binding)
                    || arguments
                        .iter()
                        .any(|argument| self.holds_an_iterator(argument))
            }
            Type::Ref(_, inner) | Type::Mut(_, inner) => self.holds_an_iterator(inner),
            Type::Function(..)
            | Type::Var(_)
            | Type::Generic(_)
            | Type::Primitive(_)
            | Type::TraitObject(_) => false,
        }
    }
}

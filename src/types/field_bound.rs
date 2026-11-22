use super::declarations::Declarations;
use super::ty::Type;
use super::unborrowed::Unborrowed;
use super::unifier::Unifier;

pub(super) struct FieldBound<'c> {
    unifier: &'c Unifier,
    declarations: &'c Declarations,
}

impl<'c> FieldBound<'c> {
    pub(super) fn new(unifier: &'c Unifier, declarations: &'c Declarations) -> Self {
        Self {
            unifier,
            declarations,
        }
    }

    pub(super) fn read(&self, owner: &Type, field: &str) -> Option<Type> {
        let field_traits = self.declarations.traits().rust_paths().field_traits();
        let binding = field_traits.bindings().trait_of(field)?;
        let Type::Generic(generic) = Unborrowed::of(owner) else {
            return None;
        };
        let bounds = self.unifier.generic_bounds(*generic).iter();
        bounds
            .filter(|bound| bound.binding == binding)
            .find_map(|bound| bound.argument.clone())
    }
}

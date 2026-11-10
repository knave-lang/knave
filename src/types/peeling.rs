use super::ty::Type;
use super::unifier::Unifier;
use crate::core::ReferenceKind;

#[derive(Clone, Copy)]
pub(super) struct ReferencePeeling<'a> {
    unifier: &'a Unifier,
}

impl<'a> ReferencePeeling<'a> {
    pub(super) fn new(unifier: &'a Unifier) -> Self {
        Self { unifier }
    }

    pub(super) fn peel(&self, ty: &Type) -> (Type, Option<ReferenceKind>) {
        let mut matched = self.unifier.resolve(ty);
        let mut mode = None;
        loop {
            match matched.clone() {
                Type::Ref(_, inner) => {
                    mode.get_or_insert(ReferenceKind::Shared);
                    matched = self.unifier.resolve(&inner);
                }
                Type::Mut(_, inner) => {
                    mode.get_or_insert(ReferenceKind::Mutable);
                    matched = self.unifier.resolve(&inner);
                }
                _ => return (matched, mode),
            }
        }
    }
}

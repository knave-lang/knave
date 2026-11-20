use super::ty::Type;
use super::unborrowed::Unborrowed;
use crate::core::PrimitiveType;

pub(super) struct ReadView<'t> {
    read: &'t Type,
}

impl<'t> ReadView<'t> {
    pub(super) fn of(read: &'t Type) -> Self {
        Self { read }
    }

    pub(super) fn is_written_as(&self, written: &Type) -> bool {
        self.views_as(written) && written.reference_depth() == self.read.reference_depth()
    }

    pub(super) fn is_used_as(&self, used: &Type) -> bool {
        let viewed = self.views_as(used) && used.reference_depth() <= self.read.reference_depth();
        viewed || Self::fits(self.read, used)
    }

    fn fits(read: &Type, used: &Type) -> bool {
        match (read, used) {
            (_, Type::Generic(_)) => true,
            (Type::Ref(_, read), Type::Ref(_, used)) | (Type::Mut(_, read), Type::Mut(_, used)) => {
                Self::fits(read, used)
            }
            (Type::Named(read, reads), Type::Named(used, uses)) => {
                read == used
                    && reads.len() == uses.len()
                    && reads
                        .iter()
                        .zip(uses)
                        .all(|(read, used)| Self::fits(read, used))
            }
            _ => read == used,
        }
    }

    fn views_as(&self, expected: &Type) -> bool {
        Self::viewed(Unborrowed::of(self.read)) == Self::viewed(Unborrowed::of(expected))
    }

    fn viewed(value: &Type) -> Type {
        match value {
            Type::Primitive(PrimitiveType::String) => Type::Primitive(PrimitiveType::Str),
            other => other.clone(),
        }
    }
}

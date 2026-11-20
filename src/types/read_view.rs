use super::ty::Type;

pub(super) struct ReadView<'t> {
    read: &'t Type,
}

impl<'t> ReadView<'t> {
    pub(super) fn of(read: &'t Type) -> Self {
        Self { read }
    }

    pub(super) fn is_used_as(&self, used: &Type) -> bool {
        Self::fits(self.read, used) || self.is_borrowed_place(used)
    }

    fn is_borrowed_place(&self, used: &Type) -> bool {
        let referent = match self.read {
            Type::Ref(_, referent) => referent.as_ref(),
            _ => return false,
        };
        used.is_unsized() && Self::fits(referent, used)
    }

    fn fits(read: &Type, used: &Type) -> bool {
        match (read, used) {
            (_, Type::Generic(_) | Type::Var(_)) | (Type::Generic(_), _) => true,
            (Type::Ref(_, read), Type::Ref(_, used)) | (Type::Mut(_, read), Type::Mut(_, used)) => {
                Self::fits(read, used)
            }
            (Type::Named(read, reads), Type::Named(used, uses)) => {
                read == used
                    && reads.len() == uses.len()
                    && (reads.iter().zip(uses)).all(|(read, used)| Self::fits(read, used))
            }
            _ => read == used,
        }
    }
}

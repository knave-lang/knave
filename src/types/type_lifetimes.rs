use super::ty::{Lifetime, Type};

pub struct TypeLifetimes<'t> {
    ty: &'t Type,
}

impl<'t> TypeLifetimes<'t> {
    pub fn of(ty: &'t Type) -> Self {
        Self { ty }
    }

    pub fn all(&self) -> Vec<Lifetime> {
        let own = match self.ty {
            Type::Function(..) => return Vec::new(),
            Type::Ref(lifetime, _) | Type::Mut(lifetime, _) => Some(*lifetime),
            _ => None,
        };
        own.into_iter()
            .chain(
                self.ty
                    .parts()
                    .into_iter()
                    .flat_map(|part| TypeLifetimes::of(part).all()),
            )
            .collect()
    }
}

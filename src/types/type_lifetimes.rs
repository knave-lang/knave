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

    pub fn names_written(&self) -> bool {
        self.all()
            .into_iter()
            .any(|lifetime| matches!(lifetime, Lifetime::Written(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::TypeLifetimes;
    use crate::core::{BindingId, PrimitiveType};
    use crate::types::{Lifetime, Type};

    struct LifetimesFixture;

    impl LifetimesFixture {
        fn reference(lifetime: Lifetime) -> Type {
            Type::Ref(lifetime, Box::new(Type::Primitive(PrimitiveType::Str)))
        }
    }

    #[test]
    fn a_reference_with_a_written_lifetime_names_it() {
        let written = LifetimesFixture::reference(Lifetime::Written(BindingId::from_index(1)));
        assert!(TypeLifetimes::of(&written).names_written());
    }

    #[test]
    fn an_elided_reference_names_no_written_lifetime() {
        let elided = LifetimesFixture::reference(Lifetime::Elided);
        assert!(!TypeLifetimes::of(&elided).names_written());
    }
}
